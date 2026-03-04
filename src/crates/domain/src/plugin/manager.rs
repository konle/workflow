use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::{TaskType, WorkflowInstanceStatus};
use crate::shared::job::TaskDispatcher;
use crate::workflow::entity::{
    NodeExecutionStatus, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};
use crate::workflow::service::WorkflowService;
use crate::task::service::TaskInstanceService;
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

    pub async fn execute_workflow(
        &self,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<()> {
        self.workflow_svc
            .start_instance(&workflow_instance.workflow_instance_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

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
        instance.nodes[node_index].status = exec_result.status.clone();
        let action = match exec_result.status {
            NodeExecutionStatus::Success => {
                if let Some(next) = instance.nodes[node_index].next_node.clone() {
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

        if let Some(job) = exec_result.dispatch_job {
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
}
