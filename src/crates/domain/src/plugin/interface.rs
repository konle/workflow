use crate::shared::workflow::TaskType;
use crate::workflow::entity::{WorkflowNodeInstanceEntity, WorkflowInstanceEntity};

pub struct ExecutionResult {}

pub trait PluginInterface {
    fn execute(&self,executor: &dyn PluginExecutor, node_instance: &mut WorkflowNodeInstanceEntity, workflow_instance: &mut WorkflowInstanceEntity) -> anyhow::Result<ExecutionResult>;
    fn plugin_type(&self) -> TaskType;
}

pub trait PluginExecutor {
    fn execute_node_instance(&self, node_instance: &mut WorkflowNodeInstanceEntity, workflow_instance: &mut WorkflowInstanceEntity) -> anyhow::Result<ExecutionResult>;
    fn partial_update_node_instance(&self, node_instance: &mut WorkflowNodeInstanceEntity) -> anyhow::Result<()>;
}