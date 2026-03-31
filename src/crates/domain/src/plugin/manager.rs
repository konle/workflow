use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::{TaskType, WorkflowInstanceStatus};
use crate::shared::job::{ExecuteWorkflowJob, TaskDispatcher, WorkflowEvent};
use crate::task::entity::{TaskInstanceEntity, TaskTemplate};
use crate::task::service::TaskInstanceService;
use crate::variable::service::VariableService;
use crate::workflow::entity::{
    NodeExecutionStatus, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};
use crate::workflow::service::WorkflowInstanceService;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

enum LoopAction {
    Advance,
    Retry,
    Done,
}

pub struct PluginManager {
    plugins: HashMap<TaskType, Box<dyn PluginInterface>>,
    workflow_instance_svc: Arc<WorkflowInstanceService>,
    task_instance_svc: Option<Arc<TaskInstanceService>>,
    variable_svc: Option<VariableService>,
    dispatcher: Arc<dyn TaskDispatcher>,
}

impl PluginManager {
    pub fn new(workflow_instance_svc: Arc<WorkflowInstanceService>, dispatcher: Arc<dyn TaskDispatcher>) -> Self {
        Self {
            plugins: HashMap::new(),
            workflow_instance_svc,
            task_instance_svc: None,
            variable_svc: None,
            dispatcher,
        }
    }

    pub fn with_variable_service(mut self, svc: VariableService) -> Self {
        self.variable_svc = Some(svc);
        self
    }

    pub fn with_task_instance_service(mut self, svc: Arc<TaskInstanceService>) -> Self {
        self.task_instance_svc = Some(svc);
        self
    }

    pub fn workflow_instance_svc(&self) -> &WorkflowInstanceService {
        &self.workflow_instance_svc
    }

    pub fn dispatcher(&self) -> Arc<dyn TaskDispatcher> {
        self.dispatcher.clone()
    }

    pub fn register(&mut self, plugin: Box<dyn PluginInterface>) {
        let task_type = plugin.plugin_type();
        self.plugins.insert(task_type, plugin);
    }

    pub async fn process_workflow_job(&self, job: crate::shared::job::ExecuteWorkflowJob, worker_id: &str) -> anyhow::Result<()> {
        let mut instance = match self.workflow_instance_svc.acquire_lock(&job.workflow_instance_id, worker_id, 10000).await {
            Ok(inst) => inst,
            Err(e) => {
                warn!(
                    workflow_instance_id = %job.workflow_instance_id,
                    worker_id = %worker_id,
                    error = %e,
                    "failed to acquire lock, skipping"
                );
                return Ok(());
            }
        };

        let result = match job.event {
            crate::shared::job::WorkflowEvent::Start => {
                if !instance.is_pending() && !instance.is_running() {
                    debug!(
                        workflow_instance_id = %job.workflow_instance_id,
                        status = ?instance.status,
                        "start ignored: instance not in pending/running"
                    );
                    Ok(())
                } else {
                    info!(workflow_instance_id = %job.workflow_instance_id, "starting workflow execution");
                    self.execute_workflow(&mut instance).await
                }
            }
            crate::shared::job::WorkflowEvent::NodeCallback { node_id, child_task_id, status, output, error_message, input } => {
                // Callback should be consumed from Await/Suspended safely through Pending boundary.
                let callback_actionable = match instance.status {
                    WorkflowInstanceStatus::Await => {
                        self.workflow_instance_svc
                            .wake_from_await(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        self.workflow_instance_svc
                            .start_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        instance = self
                            .workflow_instance_svc
                            .get_workflow_instance(instance.workflow_instance_id.clone())
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        true
                    }
                    WorkflowInstanceStatus::Suspended => {
                        self.workflow_instance_svc
                            .resume_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        self.workflow_instance_svc
                            .start_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        instance = self
                            .workflow_instance_svc
                            .get_workflow_instance(instance.workflow_instance_id.clone())
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        true
                    }
                    WorkflowInstanceStatus::Pending => {
                        self.workflow_instance_svc
                            .start_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        instance = self
                            .workflow_instance_svc
                            .get_workflow_instance(instance.workflow_instance_id.clone())
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        true
                    }
                    WorkflowInstanceStatus::Running => true,
                    _ => {
                        debug!(
                            workflow_instance_id = %job.workflow_instance_id,
                            status = ?instance.status,
                            "node callback ignored: instance not in await/suspended/running"
                        );
                        false
                    }
                };
                if !callback_actionable {
                    Ok(())
                } else {
                debug!(
                    workflow_instance_id = %job.workflow_instance_id,
                    node_id = %node_id,
                    child_task_id = %child_task_id,
                    callback_status = ?status,
                    "processing node callback"
                );
                let mut cb_status = status.clone();
                let mut cb_output = output.clone();
                let mut cb_error_message = error_message.clone();
                let mut cb_input = input.clone();

                // C: callback fallback - if callback payload misses fields, load from task_instances.
                if let Some(task_svc) = &self.task_instance_svc {
                    if let Ok(task_inst) = task_svc.get_task_instance_entity(child_task_id.clone()).await {
                        if cb_input.is_none() {
                            cb_input = task_inst.input.clone();
                        }
                        if cb_output.is_none() {
                            cb_output = task_inst.output.clone();
                        }
                        if cb_error_message.is_none() {
                            cb_error_message = task_inst.error_message.clone();
                        }
                        cb_status = match task_inst.task_status {
                            crate::shared::workflow::TaskInstanceStatus::Completed => NodeExecutionStatus::Success,
                            crate::shared::workflow::TaskInstanceStatus::Failed => NodeExecutionStatus::Failed,
                            crate::shared::workflow::TaskInstanceStatus::Running => NodeExecutionStatus::Running,
                            crate::shared::workflow::TaskInstanceStatus::Canceled => NodeExecutionStatus::Failed,
                            _ => cb_status,
                        };
                    }
                }
                let mut callback_result = Ok(());
                if let Some(node_index) = instance.nodes.iter().position(|n| n.node_id == node_id) {
                    instance.current_node = node_id.clone();
                    
                    let mut node = instance.nodes[node_index].clone();
                    match self.handle_node_callback(&mut node, &mut instance, &child_task_id, &cb_status, &cb_output, &cb_error_message, &cb_input).await {
                        Ok(exec_result) => {
                            instance.nodes[node_index] = node;
                            match self.apply_exec_result(&mut instance, node_index, exec_result).await {
                                Ok(action) => {
                                    match action {
                                        LoopAction::Advance | LoopAction::Retry => {
                                            callback_result = self.execute_workflow_loop(&mut instance).await;
                                        },
                                        LoopAction::Done => {}
                                    }
                                }
                                Err(e) => callback_result = Err(e),
                            }
                        }
                        Err(e) => {
                            error!(
                                workflow_instance_id = %job.workflow_instance_id,
                                node_id = %node_id,
                                error = %e,
                                "node callback handling failed"
                            );
                            callback_result = Err(e);
                        }
                    }
                }
                callback_result
                }
            }
        };

        if result.is_ok() {
            if let Err(e) = self.notify_parent_if_needed(&job.workflow_instance_id).await {
                warn!(
                    workflow_instance_id = %job.workflow_instance_id,
                    error = %e,
                    "failed to notify parent workflow"
                );
            }
        }

        if let Err(e) = self.workflow_instance_svc.release_lock(&job.workflow_instance_id, worker_id).await {
            warn!(
                workflow_instance_id = %job.workflow_instance_id,
                error = %e,
                "failed to release lock"
            );
        }

        result
    }

    pub async fn execute_workflow(
        &self,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<()> {
        let latest = self.workflow_instance_svc
            .get_workflow_instance(workflow_instance.workflow_instance_id.clone())
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        match latest.status {
            WorkflowInstanceStatus::Pending => {
                self.workflow_instance_svc
                    .start_instance(&workflow_instance.workflow_instance_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
            WorkflowInstanceStatus::Running => {
                // Idempotent Start: when the same Start event is retried, the instance may
                // already be Running. In that case we skip state transition and continue loop.
            }
            _ => {
                debug!(
                    workflow_instance_id = %workflow_instance.workflow_instance_id,
                    status = ?latest.status,
                    "start event ignored for non-actionable workflow status"
                );
                return Ok(());
            }
        }

        self.execute_workflow_loop(workflow_instance).await
    }

    async fn execute_workflow_loop(
        &self,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<()> {

        loop {
            debug!( workflow_instance = %workflow_instance, "executing workflow loop");
            let mut instance = self
                .workflow_instance_svc
                .get_workflow_instance(workflow_instance.workflow_instance_id.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            if !instance.is_running() {
                debug!(
                    workflow_instance_id = %instance.workflow_instance_id,
                    status = ?instance.status,
                    "instance not in running state, returning"
                );
                return Ok(());
            }

            let current_node_id = instance.get_current_node();
            let node_index = instance
                .nodes
                .iter()
                .position(|n| n.node_id == current_node_id)
                .ok_or_else(|| {
                    error!(
                        workflow_instance_id = %instance.workflow_instance_id,
                        node_id = %current_node_id,
                        "node not found in instance"
                    );
                    anyhow::anyhow!("node not found: {}", current_node_id)
                })?;

            let node_status = instance.nodes[node_index].status.clone();

            match node_status {
                NodeExecutionStatus::Success => {
                    if let Some(next) = instance.nodes[node_index].next_node.clone() {
                        instance.current_node = next;
                        self.save_instance_and_bump_epoch(&mut instance).await?;
                        continue;
                    } else {
                        info!(workflow_instance_id = %instance.workflow_instance_id, "workflow completed");
                        self.workflow_instance_svc
                            .complete_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        return Ok(());
                    }
                }
                NodeExecutionStatus::Failed => {
                    error!(
                        workflow_instance_id = %instance.workflow_instance_id,
                        node_id = %current_node_id,
                        "workflow failed at node"
                    );
                    self.workflow_instance_svc
                        .fail_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    return Ok(());
                }
                NodeExecutionStatus::Suspended | NodeExecutionStatus::Running => {
                    return Ok(());
                }
                NodeExecutionStatus::Pending => {
                    instance.nodes[node_index].status = NodeExecutionStatus::Running;
                    self.save_instance_and_bump_epoch(&mut instance).await?;

                    match self.run_node(&mut instance, node_index).await? {
                        LoopAction::Advance | LoopAction::Retry => continue,
                        LoopAction::Done => return Ok(()),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    async fn run_node(
        &self,
        instance: &mut WorkflowInstanceEntity,
        node_index: usize,
    ) -> anyhow::Result<LoopAction> {
        let mut node = instance.nodes[node_index].clone();

        if let Some(ref var_svc) = self.variable_svc {
            match var_svc.resolve_variables(
                &instance.tenant_id,
                &instance.workflow_meta_id,
                &instance.context,
                &node.context,
            ).await {
                Ok(merged) => node.context = merged,
                Err(e) => warn!(
                    workflow_instance_id = %instance.workflow_instance_id,
                    node_id = %node.node_id,
                    error = %e,
                    "variable resolution failed, using raw context"
                ),
            }
        }

        let result = self.execute_node_instance(&mut node, instance).await;
        instance.nodes[node_index] = node;
        debug!( workflow_instance = %instance, "node execution result: %result");
        let exec_result = match result {
            Ok(r) => r,
            Err(e) => {
                error!(
                    workflow_instance_id = %instance.workflow_instance_id,
                    node_id = %instance.nodes[node_index].node_id,
                    error = %e,
                    "node execution failed"
                );
                instance.nodes[node_index].error_message = Some(e.to_string());
                ExecutionResult::failed()
            }
        };
        let action = self.apply_exec_result(instance, node_index, exec_result).await?;
        Ok(action)
    }

    async fn apply_exec_result(
        &self,
        instance: &mut WorkflowInstanceEntity,
        node_index: usize,
        exec_result: ExecutionResult,
    ) -> anyhow::Result<LoopAction> {
        instance.nodes[node_index].status = exec_result.status.clone();
        let action = match exec_result.status {
            NodeExecutionStatus::Success => {
                if let Some(jump_to_node) = exec_result.jump_to_node {
                    instance.current_node = jump_to_node.clone();
                    instance.nodes[node_index].next_node = Some(jump_to_node);
                    LoopAction::Advance
                }
                else if let Some(next) = instance.nodes[node_index].next_node.clone() {
                    instance.current_node = next;
                    LoopAction::Advance
                } else {
                    self.workflow_instance_svc
                        .complete_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Completed;
                    LoopAction::Done
                }
            }
            NodeExecutionStatus::Failed => {
                self.workflow_instance_svc
                    .fail_instance(&instance.workflow_instance_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                instance.status = WorkflowInstanceStatus::Failed;
                LoopAction::Done
            }
            NodeExecutionStatus::Pending | NodeExecutionStatus::Suspended => {
                if !exec_result.dispatch_jobs.is_empty() || !exec_result.dispatch_workflow_jobs.is_empty() {
                    // Async dispatch path: workflow yields CPU and waits callback.
                    self.workflow_instance_svc
                        .await_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Await;
                } else {
                    // Manual intervention path (e.g. approval waiting for human action).
                    self.workflow_instance_svc
                        .suspend_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Suspended;
                }
                LoopAction::Done
            }
            _ => LoopAction::Retry,
        };

        self.save_instance_and_bump_epoch(instance).await?;

        for job in &exec_result.dispatch_jobs {
            self.ensure_task_instance_for_job(instance, node_index, job).await?;
        }

        for job in exec_result.dispatch_jobs {
            if let Err(e) = self.dispatcher.dispatch_task(job.clone()).await {
                error!(task_instance_id = %job.task_instance_id, error = %e, "failed to dispatch task");
                return Err(e.into());
            }
        }
        for job in exec_result.dispatch_workflow_jobs {
            if let Err(e) = self.dispatcher.dispatch_workflow(job.clone()).await {
                error!(workflow_instance_id = %job.workflow_instance_id, error = %e, "failed to dispatch workflow");
                return Err(e.into());
            }
        }

        Ok(action)
    }

    async fn save_instance_and_bump_epoch(&self, instance: &mut WorkflowInstanceEntity) -> anyhow::Result<()> {
        self.workflow_instance_svc
            .save_workflow_instance(instance)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        instance.epoch += 1;
        instance.updated_at = chrono::Utc::now();
        Ok(())
    }

    async fn ensure_task_instance_for_job(
        &self,
        instance: &WorkflowInstanceEntity,
        node_index: usize,
        job: &crate::shared::job::ExecuteTaskJob,
    ) -> anyhow::Result<()> {
        let Some(task_svc) = &self.task_instance_svc else {
            return Ok(());
        };

        if task_svc.get_task_instance_entity(job.task_instance_id.clone()).await.is_ok() {
            return Ok(());
        }

        let now = chrono::Utc::now();
        let parent = &instance.nodes[node_index].task_instance;
        let (child_template, child_task_type) = match &parent.task_template {
            TaskTemplate::Parallel(pt) => {
                let inner = (*pt.task_template).clone();
                let tt = inner.task_type();
                (inner, tt)
            }
            TaskTemplate::ForkJoin(fj) => {
                let idx = job
                    .caller_context
                    .as_ref()
                    .and_then(|c| c.item_index)
                    .ok_or_else(|| {
                        anyhow::anyhow!("ForkJoin dispatch job missing item_index in caller_context")
                    })?;
                let item = fj.tasks.get(idx).ok_or_else(|| {
                    anyhow::anyhow!(
                        "ForkJoin item_index {} out of range (len {})",
                        idx,
                        fj.tasks.len()
                    )
                })?;
                let inner = item.task_template.clone();
                let tt = inner.task_type();
                (inner, tt)
            }
            _ => (parent.task_template.clone(), parent.task_type.clone()),
        };

        let mut task_instance: TaskInstanceEntity = parent.clone();
        task_instance.task_template = child_template;
        task_instance.task_type = child_task_type;
        task_instance.id = job.task_instance_id.clone();
        task_instance.task_id = parent.task_id.clone();
        task_instance.task_instance_id = job.task_instance_id.clone();
        task_instance.tenant_id = job.tenant_id.clone();
        task_instance.caller_context = job.caller_context.clone();
        task_instance.created_at = now;
        task_instance.updated_at = now;
        task_instance.input = None;
        task_instance.output = None;
        task_instance.error_message = None;
        task_instance.execution_duration = None;
        task_instance.task_status = crate::shared::workflow::TaskInstanceStatus::Pending;

        task_svc
            .create_task_instance_entity(task_instance)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    async fn notify_parent_if_needed(&self, workflow_instance_id: &str) -> anyhow::Result<()> {
        let instance = self.workflow_instance_svc
            .get_workflow_instance(workflow_instance_id.to_string())
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let is_terminal = matches!(
            instance.status,
            WorkflowInstanceStatus::Completed | WorkflowInstanceStatus::Failed
        );

        if !is_terminal {
            return Ok(());
        }

        if let Some(parent_ctx) = &instance.parent_context {
            let status = match instance.status {
                WorkflowInstanceStatus::Completed => NodeExecutionStatus::Success,
                _ => NodeExecutionStatus::Failed,
            };
            info!(
                child_workflow_id = %workflow_instance_id,
                parent_workflow_id = %parent_ctx.workflow_instance_id,
                status = ?status,
                "notifying parent workflow"
            );
            self.dispatcher.dispatch_workflow(ExecuteWorkflowJob {
                workflow_instance_id: parent_ctx.workflow_instance_id.clone(),
                tenant_id: instance.tenant_id.clone(),
                event: WorkflowEvent::NodeCallback {
                    node_id: parent_ctx.node_id.clone(),
                    child_task_id: instance.workflow_instance_id.clone(),
                    status,
                    output: Some(instance.context.clone()),
                    error_message: None,
                    input: None,
                },
            }).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl PluginExecutor for PluginManager {
    async fn execute_node_instance(
        &self,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let plugin = self.plugins.get(&node_instance.node_type).ok_or_else(|| {
            error!(
                node_type = ?node_instance.node_type,
                node_id = %node_instance.node_id,
                "no plugin registered for node type"
            );
            anyhow::anyhow!(
                "no plugin registered for task type: {:?}",
                node_instance.node_type
            )
        })?;

        plugin.execute(self, node_instance, workflow_instance).await
    }

    async fn handle_node_callback(
        &self,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
        child_task_id: &str,
        status: &NodeExecutionStatus,
        output: &Option<serde_json::Value>,
        error_message: &Option<String>,
        input: &Option<serde_json::Value>,
    ) -> anyhow::Result<ExecutionResult> {
        let plugin = self.plugins.get(&node_instance.node_type).ok_or_else(|| {
            error!(
                node_type = ?node_instance.node_type,
                node_id = %node_instance.node_id,
                "no plugin registered for callback"
            );
            anyhow::anyhow!(
                "no plugin registered for task type: {:?}",
                node_instance.node_type
            )
        })?;

        plugin.handle_callback(self, node_instance, workflow_instance, child_task_id, status, output, error_message, input).await
    }
}
