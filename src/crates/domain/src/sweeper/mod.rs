use std::sync::Arc;
use tracing::{info, warn, debug, error};

use crate::approval::service::ApprovalService;
use crate::shared::job::{
    ExecuteTaskJob, ExecuteWorkflowJob, TaskDispatcher, WorkflowCallerContext, WorkflowEvent,
};
use crate::shared::workflow::{TaskInstanceStatus, WorkflowInstanceStatus};
use crate::task::service::TaskInstanceService;
use crate::workflow::entity::workflow_definition::{NodeExecutionStatus, WorkflowInstanceEntity};
use crate::workflow::service::WorkflowInstanceService;

#[derive(Debug, Clone)]
pub struct SweeperConfig {
    pub interval_secs: u64,
    pub max_recover_per_cycle: u32,
}

impl Default for SweeperConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            max_recover_per_cycle: 10,
        }
    }
}

pub struct Sweeper {
    workflow_instance_svc: Arc<WorkflowInstanceService>,
    task_instance_svc: Arc<TaskInstanceService>,
    approval_svc: Option<ApprovalService>,
    dispatcher: Arc<dyn TaskDispatcher>,
    config: SweeperConfig,
}

impl Sweeper {
    pub fn new(
        workflow_instance_svc: Arc<WorkflowInstanceService>,
        task_instance_svc: Arc<TaskInstanceService>,
        dispatcher: Arc<dyn TaskDispatcher>,
        config: SweeperConfig,
    ) -> Self {
        Self {
            workflow_instance_svc,
            task_instance_svc,
            approval_svc: None,
            dispatcher,
            config,
        }
    }

    pub fn with_approval_service(mut self, svc: ApprovalService) -> Self {
        self.approval_svc = Some(svc);
        self
    }

    pub async fn run_cycle(&self) {
        let zombies = match self
            .workflow_instance_svc
            .scan_zombie_instances(self.config.max_recover_per_cycle)
            .await
        {
            Ok(z) => z,
            Err(e) => {
                error!(error = %e, "sweeper: failed to scan zombie instances");
                return;
            }
        };

        if zombies.is_empty() {
            debug!("sweeper: no zombie instances found");
            return;
        }

        let mut recovered_running = 0u32;
        let mut recovered_await = 0u32;
        let mut skipped_cas = 0u32;

        for instance in &zombies {
            let id = &instance.workflow_instance_id;
            let epoch = instance.epoch;

            if let Err(_) = self
                .workflow_instance_svc
                .force_clear_lock(id, epoch)
                .await
            {
                debug!(workflow_instance_id = %id, epoch, "sweeper: CAS failed, skipping");
                skipped_cas += 1;
                continue;
            }

            match instance.status {
                WorkflowInstanceStatus::Running => {
                    match self.recover_running(instance).await {
                        Ok(_) => recovered_running += 1,
                        Err(e) => warn!(
                            workflow_instance_id = %id,
                            error = %e,
                            "sweeper: failed to recover running instance"
                        ),
                    }
                }
                WorkflowInstanceStatus::Await => {
                    match self.recover_await(instance).await {
                        Ok(_) => recovered_await += 1,
                        Err(e) => warn!(
                            workflow_instance_id = %id,
                            error = %e,
                            "sweeper: failed to recover await instance"
                        ),
                    }
                }
                _ => {}
            }
        }

        let expired_approvals = self.sweep_expired_approvals().await;

        info!(
            scanned = zombies.len(),
            recovered_running,
            recovered_await,
            skipped_cas,
            expired_approvals,
            "sweeper cycle completed"
        );
    }

    /// Phase 1: Running → Pending → dispatch Start
    async fn recover_running(&self, instance: &WorkflowInstanceEntity) -> anyhow::Result<()> {
        let id = &instance.workflow_instance_id;

        self.workflow_instance_svc
            .transfer_status_unchecked(id, &WorkflowInstanceStatus::Pending)
            .await
            .map_err(|e| anyhow::anyhow!("Running→Pending failed: {e}"))?;

        self.workflow_instance_svc
            .start_instance(id)
            .await
            .map_err(|e| anyhow::anyhow!("Pending→Running failed: {e}"))?;

        self.dispatcher
            .dispatch_workflow(ExecuteWorkflowJob {
                workflow_instance_id: id.clone(),
                tenant_id: instance.tenant_id.clone(),
                event: WorkflowEvent::Start,
            })
            .await?;

        info!(
            workflow_instance_id = %id,
            action = "restarted",
            "sweeper recovered running instance"
        );
        Ok(())
    }

    /// Phase 2: Await — look up child tasks, supplement missing callbacks or re-dispatch
    async fn recover_await(&self, instance: &WorkflowInstanceEntity) -> anyhow::Result<()> {
        let current_node_id = &instance.current_node;

        let node = instance
            .nodes
            .iter()
            .find(|n| n.node_id == *current_node_id)
            .ok_or_else(|| anyhow::anyhow!("current node {} not found", current_node_id))?;

        let task_template = &node.task_instance.task_template;

        use crate::task::entity::TaskTemplate;

        match task_template {
            TaskTemplate::Parallel(_) | TaskTemplate::ForkJoin(_) => {
                self.recover_await_container(instance, node).await
            }
            _ => {
                self.recover_await_single(instance, node).await
            }
        }
    }

    /// Recover a single-child Await node (Http, SubWorkflow, etc.)
    async fn recover_await_single(
        &self,
        instance: &WorkflowInstanceEntity,
        node: &crate::workflow::entity::workflow_definition::WorkflowNodeInstanceEntity,
    ) -> anyhow::Result<()> {
        let id = &instance.workflow_instance_id;
        let child_task_id = format!("{}-{}", id, node.node_id);

        match self.task_instance_svc.get_task_instance_entity(child_task_id.clone()).await {
            Ok(task) => {
                match task.task_status {
                    TaskInstanceStatus::Completed | TaskInstanceStatus::Failed => {
                        let status = if task.task_status == TaskInstanceStatus::Completed {
                            NodeExecutionStatus::Success
                        } else {
                            NodeExecutionStatus::Failed
                        };

                        self.supplement_callback(
                            instance,
                            &node.node_id,
                            &child_task_id,
                            status,
                            task.output.clone(),
                            task.error_message.clone(),
                            task.input.clone(),
                        )
                        .await?;

                        info!(
                            workflow_instance_id = %id,
                            child_task_id = %child_task_id,
                            action = "callback_supplemented",
                            "sweeper supplemented missing callback"
                        );
                    }
                    _ => {
                        self.redispatch_task(instance, &child_task_id, &node.node_id).await?;
                        info!(
                            workflow_instance_id = %id,
                            child_task_id = %child_task_id,
                            action = "task_redispatched",
                            "sweeper redispatched stale task"
                        );
                    }
                }
            }
            Err(_) => {
                self.recover_running(instance).await?;
            }
        }
        Ok(())
    }

    /// Recover a container Await node (Parallel / ForkJoin)
    async fn recover_await_container(
        &self,
        instance: &WorkflowInstanceEntity,
        node: &crate::workflow::entity::workflow_definition::WorkflowNodeInstanceEntity,
    ) -> anyhow::Result<()> {
        let id = &instance.workflow_instance_id;
        let state = node.task_instance.output.as_ref()
            .ok_or_else(|| anyhow::anyhow!("no state in parallel/forkjoin node output"))?;

        let total_items = state["total_items"].as_u64().unwrap_or(0) as usize;
        let dispatched_count = state["dispatched_count"].as_u64().unwrap_or(0) as usize;
        let success_count = state["success_count"].as_u64().unwrap_or(0) as usize;
        let failed_count = state["failed_count"].as_u64().unwrap_or(0) as usize;
        let known_completed = success_count + failed_count;

        let mut supplemented = 0u32;
        let mut redispatched = 0u32;

        for index in 0..dispatched_count {
            let child_task_id = format!("{}-{}-{}", id, node.node_id, index);

            let task = match self.task_instance_svc.get_task_instance_entity(child_task_id.clone()).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            match task.task_status {
                TaskInstanceStatus::Completed => {
                    // Only supplement if this callback was likely lost
                    // (we can't perfectly deduplicate here; Phase 3 adds proper dedup)
                    if known_completed < dispatched_count {
                        self.supplement_callback(
                            instance,
                            &node.node_id,
                            &child_task_id,
                            NodeExecutionStatus::Success,
                            task.output.clone(),
                            task.error_message.clone(),
                            task.input.clone(),
                        )
                        .await?;
                        supplemented += 1;
                    }
                }
                TaskInstanceStatus::Failed => {
                    if known_completed < dispatched_count {
                        self.supplement_callback(
                            instance,
                            &node.node_id,
                            &child_task_id,
                            NodeExecutionStatus::Failed,
                            task.output.clone(),
                            task.error_message.clone(),
                            task.input.clone(),
                        )
                        .await?;
                        supplemented += 1;
                    }
                }
                TaskInstanceStatus::Pending | TaskInstanceStatus::Running => {
                    self.redispatch_task(instance, &child_task_id, &node.node_id).await?;
                    redispatched += 1;
                }
                _ => {}
            }
        }

        info!(
            workflow_instance_id = %id,
            node_id = %node.node_id,
            supplemented,
            redispatched,
            total_items,
            dispatched_count,
            known_completed,
            "sweeper recovered container node"
        );
        Ok(())
    }

    async fn supplement_callback(
        &self,
        instance: &WorkflowInstanceEntity,
        node_id: &str,
        child_task_id: &str,
        status: NodeExecutionStatus,
        output: Option<serde_json::Value>,
        error_message: Option<String>,
        input: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        self.dispatcher
            .dispatch_workflow(ExecuteWorkflowJob {
                workflow_instance_id: instance.workflow_instance_id.clone(),
                tenant_id: instance.tenant_id.clone(),
                event: WorkflowEvent::NodeCallback {
                    node_id: node_id.to_string(),
                    child_task_id: child_task_id.to_string(),
                    status,
                    output,
                    error_message,
                    input,
                },
            })
            .await
    }

    async fn redispatch_task(
        &self,
        instance: &WorkflowInstanceEntity,
        task_instance_id: &str,
        node_id: &str,
    ) -> anyhow::Result<()> {
        self.dispatcher
            .dispatch_task(ExecuteTaskJob {
                task_instance_id: task_instance_id.to_string(),
                tenant_id: instance.tenant_id.clone(),
                caller_context: Some(WorkflowCallerContext {
                    workflow_instance_id: instance.workflow_instance_id.clone(),
                    node_id: node_id.to_string(),
                    parent_task_instance_id: None,
                    item_index: None,
                }),
            })
            .await
    }

    /// Phase 3: Scan expired approval instances and reject them + send NodeCallback(Failed).
    async fn sweep_expired_approvals(&self) -> u32 {
        let approval_svc = match &self.approval_svc {
            Some(s) => s,
            None => return 0,
        };

        let expired = match approval_svc
            .scan_expired_approvals(self.config.max_recover_per_cycle)
            .await
        {
            Ok(e) => e,
            Err(e) => {
                error!(error = %e, "sweeper: failed to scan expired approvals");
                return 0;
            }
        };

        let mut count = 0u32;
        for approval in &expired {
            if let Err(e) = approval_svc.expire_approval(approval).await {
                warn!(
                    approval_id = %approval.id,
                    error = %e,
                    "sweeper: failed to expire approval"
                );
                continue;
            }

            if let Err(e) = self
                .dispatcher
                .dispatch_workflow(ExecuteWorkflowJob {
                    workflow_instance_id: approval.workflow_instance_id.clone(),
                    tenant_id: approval.tenant_id.clone(),
                    event: WorkflowEvent::NodeCallback {
                        node_id: approval.node_id.clone(),
                        child_task_id: approval.id.clone(),
                        status: NodeExecutionStatus::Failed,
                        output: Some(serde_json::json!({
                            "approval_expired": true,
                            "expires_at": approval.expires_at,
                        })),
                        error_message: Some("approval expired".to_string()),
                        input: None,
                    },
                })
                .await
            {
                warn!(
                    approval_id = %approval.id,
                    error = %e,
                    "sweeper: failed to dispatch expired approval callback"
                );
                continue;
            }

            info!(
                approval_id = %approval.id,
                workflow_instance_id = %approval.workflow_instance_id,
                node_id = %approval.node_id,
                "sweeper: expired approval → rejected + callback dispatched"
            );
            count += 1;
        }
        count
    }
}
