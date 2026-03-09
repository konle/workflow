use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::{TaskType, WorkflowInstanceStatus};
use crate::shared::job::TaskDispatcher;
use crate::workflow::entity::{
    NodeExecutionStatus, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};
use crate::workflow::service::WorkflowService;
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
    workflow_svc: Arc<WorkflowService>,
    dispatcher: Arc<dyn TaskDispatcher>,
}

impl PluginManager {
    pub fn new(workflow_svc: Arc<WorkflowService>, dispatcher: Arc<dyn TaskDispatcher>) -> Self {
        Self {
            plugins: HashMap::new(),
            workflow_svc,
            dispatcher,
        }
    }

    pub fn workflow_svc(&self) -> &WorkflowService {
        &self.workflow_svc
    }

    pub fn dispatcher(&self) -> Arc<dyn TaskDispatcher> {
        self.dispatcher.clone()
    }

    pub fn register(&mut self, plugin: Box<dyn PluginInterface>) {
        let task_type = plugin.plugin_type();
        self.plugins.insert(task_type, plugin);
    }

    pub async fn process_workflow_job(&self, job: crate::shared::job::ExecuteWorkflowJob, worker_id: &str) -> anyhow::Result<()> {
        let mut instance = match self.workflow_svc.acquire_lock(&job.workflow_instance_id, worker_id, 10000).await {
            Ok(inst) => inst,
            Err(e) => {
                println!("Failed to acquire lock for {}: {}, skipping or retrying later...", job.workflow_instance_id, e);
                return Ok(()); // Lock not acquired, event is effectively ignored or would be retried based on queue settings
            }
        };

        if !instance.is_pending() && !instance.is_running() {
            // Must release lock if we bail out early
            let _ = self.workflow_svc.release_lock(&instance.workflow_instance_id, worker_id).await;
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

        // Always attempt to release the lock when done
        let _ = self.workflow_svc.release_lock(&job.workflow_instance_id, worker_id).await;

        result
    }

    pub async fn execute_workflow(
        &self,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<()> {
        self.workflow_svc
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
                .workflow_svc
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
                        self.workflow_svc
                            .save_workflow_instance(&instance)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        continue;
                    } else {
                        self.workflow_svc
                            .complete_instance(&instance.workflow_instance_id)
                            .await
                            .map_err(|e| anyhow::anyhow!(e))?;
                        return Ok(());
                    }
                }
                NodeExecutionStatus::Failed => {
                    self.workflow_svc
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
                    self.workflow_svc
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
                    self.workflow_svc
                        .complete_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Completed;
                    LoopAction::Done
                }
            }
            NodeExecutionStatus::Failed => {
                self.workflow_svc
                    .fail_instance(&instance.workflow_instance_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                instance.status = WorkflowInstanceStatus::Failed;
                LoopAction::Done
            }
            NodeExecutionStatus::Pending | NodeExecutionStatus::Suspended => {
                self.workflow_svc
                    .suspend_instance(&instance.workflow_instance_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                instance.status = WorkflowInstanceStatus::Suspended;
                LoopAction::Done
            }
            _ => LoopAction::Retry,
        };

        self.workflow_svc
            .save_workflow_instance(instance)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        for job in exec_result.dispatch_jobs {
            self.dispatcher.dispatch_task(job).await?;
        }

        Ok(action)
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
