use async_trait::async_trait;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::workflow::entity::workflow_definition::{
    WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct LlmPlugin {}

impl LlmPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for LlmPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let job = ExecuteTaskJob {
            task_instance_id: format!(
                "{}-{}",
                workflow_instance.workflow_instance_id, node_instance.node_id
            ),
            tenant_id: workflow_instance.tenant_id.clone(),
            caller_context: Some(WorkflowCallerContext {
                workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                node_id: node_instance.node_id.clone(),
                parent_task_instance_id: None,
                item_index: None,
            }),
        };

        Ok(ExecutionResult::async_dispatch(job))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Llm
    }
}
