use async_trait::async_trait;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::workflow::entity::{
    WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct HttpPlugin {}

impl HttpPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for HttpPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        // Here we just return Pending to suspend the workflow.
        // The actual HTTP execution is handled by the task worker.
        // We will need to push an ExecuteTaskJob to the task queue here,
        // but since the plugin doesn't have direct access to the queue yet,
        // we'll need to inject a task dispatcher or handle it in the manager.
        // For now, we just return pending.
        Ok(ExecutionResult::pending())
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Http
    }
}
