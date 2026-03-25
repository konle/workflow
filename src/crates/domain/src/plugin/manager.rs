use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::{TaskType, WorkflowInstanceStatus};
use crate::shared::job::{ExecuteWorkflowJob, TaskDispatcher, WorkflowEvent};
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
    variable_svc: Option<VariableService>,
    dispatcher: Arc<dyn TaskDispatcher>,
}

impl PluginManager {
    pub fn new(workflow_instance_svc: Arc<WorkflowInstanceService>, dispatcher: Arc<dyn TaskDispatcher>) -> Self {
        Self {
            plugins: HashMap::new(),
            workflow_instance_svc,
            variable_svc: None,
            dispatcher,
        }
    }

    pub fn with_variable_service(mut self, svc: VariableService) -> Self {
        self.variable_svc = Some(svc);
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

        if !instance.is_pending() && !instance.is_running() {
            debug!(
                workflow_instance_id = %job.workflow_instance_id,
                status = ?instance.status,
                "instance not in actionable state, releasing lock"
            );
            let _ = self.workflow_instance_svc.release_lock(&instance.workflow_instance_id, worker_id).await;
            return Ok(());
        }

        let result = match job.event {
            crate::shared::job::WorkflowEvent::Start => {
                info!(workflow_instance_id = %job.workflow_instance_id, "starting workflow execution");
                self.execute_workflow(&mut instance).await
            }
            crate::shared::job::WorkflowEvent::NodeCallback { node_id, child_task_id, status, output, error_message, input } => {
                debug!(
                    workflow_instance_id = %job.workflow_instance_id,
                    node_id = %node_id,
                    child_task_id = %child_task_id,
                    callback_status = ?status,
                    "processing node callback"
                );
                let mut callback_result = Ok(());
                if let Some(node_index) = instance.nodes.iter().position(|n| n.node_id == node_id) {
                    instance.current_node = node_id.clone();
                    
                    let mut node = instance.nodes[node_index].clone();
                    match self.handle_node_callback(&mut node, &mut instance, &child_task_id, &status, &output, &error_message, &input).await {
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
        self.workflow_instance_svc
            .start_instance(&workflow_instance.workflow_instance_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        self.execute_workflow_loop(workflow_instance).await
    }

    async fn execute_workflow_loop(
        &self,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<()> {

        loop {
            let mut instance = self
                .workflow_instance_svc
                .get_workflow_instance(workflow_instance.workflow_instance_id.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            if !instance.is_pending() {
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
                        self.workflow_instance_svc
                            .save_workflow_instance(&instance)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
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
                    self.workflow_instance_svc
                        .save_workflow_instance(&instance)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;

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
                    instance.current_node = jump_to_node;
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
                self.workflow_instance_svc
                    .suspend_instance(&instance.workflow_instance_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                instance.status = WorkflowInstanceStatus::Suspended;
                LoopAction::Done
            }
            _ => LoopAction::Retry,
        };

        self.workflow_instance_svc
            .save_workflow_instance(instance)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

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
