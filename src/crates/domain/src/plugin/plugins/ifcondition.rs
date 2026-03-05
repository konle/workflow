use async_trait::async_trait;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;

use crate::workflow::entity::{
    WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct IfConditionPlugin {}

impl IfConditionPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for IfConditionPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        _node_instance: &mut WorkflowNodeInstanceEntity,
        _workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        
        Ok(ExecutionResult::success(None))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::IfCondition
    }
}