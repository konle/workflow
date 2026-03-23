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
                println!("Failed to acquire lock for {}: {}, skipping or retrying later...", job.workflow_instance_id, e);
                return Ok(()); // Lock not acquired, event is effectively ignored or would be retried based on queue settings
            }
        };

        if !instance.is_pending() && !instance.is_running() {
            // Must release lock if we bail out early
            let _ = self.workflow_instance_svc.release_lock(&instance.workflow_instance_id, worker_id).await;
            return Ok(());
        }

        let result = match job.event {
            crate::shared::job::WorkflowEvent::Start => {
                self.execute_workflow(&mut instance).await
            }
            crate::shared::job::WorkflowEvent::NodeCallback { node_id, child_task_id, status, output, error_message, input } => {
                let mut callback_result = Ok(());
                if let Some(node_index) = instance.nodes.iter().position(|n| n.node_id == node_id) {
                    // Update current_node to ensure execute_workflow loop starts from here if it's not already
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
                                        LoopAction::Done => {
                                            // Completed or suspended, stop
                                        }
                                    }
                                }
                                Err(e) => callback_result = Err(e),
                            }
                        }
                        Err(e) => callback_result = Err(e),
                    }
                }
                callback_result
            }
        };

        // If this workflow reached a terminal state, notify parent (if sub-workflow)
        if result.is_ok() {
            let _ = self.notify_parent_if_needed(&job.workflow_instance_id).await;
        }

        // Always attempt to release the lock when done
        let _ = self.workflow_instance_svc.release_lock(&job.workflow_instance_id, worker_id).await;

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
                .ok_or_else(|| anyhow::anyhow!("node not found: {}", current_node_id))?;

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
                        self.workflow_instance_svc
                            .complete_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        return Ok(());
                    }
                }
                NodeExecutionStatus::Failed => {
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
                Err(e) => println!("Warning: variable resolution failed: {}, using raw context", e),
            }
        }

        let result = self.execute_node_instance(&mut node, instance).await;
        instance.nodes[node_index] = node;

        let exec_result = match result {
            Ok(r) => r,
            Err(e) => {
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
                // 如果插件动态指定了下一跳（If 节点算出来的），优先走跳转
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
            self.dispatcher.dispatch_task(job).await?;
        }
        for job in exec_result.dispatch_workflow_jobs {
            self.dispatcher.dispatch_workflow(job).await?;
        }

        Ok(action)
    }

    /// If this workflow has a parent (sub-workflow), dispatch a NodeCallback to the parent.
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
            anyhow::anyhow!(
                "no plugin registered for task type: {:?}",
                node_instance.node_type
            )
        })?;

        plugin.handle_callback(self, node_instance, workflow_instance, child_task_id, status, output, error_message, input).await
    }
}
