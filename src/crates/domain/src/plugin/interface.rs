use async_trait::async_trait;
use crate::shared::workflow::TaskType;
use crate::workflow::entity::{WorkflowNodeInstanceEntity, WorkflowInstanceEntity, NodeExecutionStatus};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: NodeExecutionStatus,
}

impl ExecutionResult {
    pub fn success() -> Self {
        Self { status: NodeExecutionStatus::Success }
    }

    pub fn failed() -> Self {
        Self { status: NodeExecutionStatus::Failed }
    }

    pub fn suspended() -> Self {
        Self { status: NodeExecutionStatus::Suspended }
    }

    pub fn pending() -> Self {
        Self { status: NodeExecutionStatus::Pending }
    }
}

#[async_trait]
pub trait PluginInterface: Send + Sync {
    async fn execute(
        &self,
        executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult>;

    fn plugin_type(&self) -> TaskType;
}

#[async_trait]
pub trait PluginExecutor: Send + Sync {
    async fn execute_node_instance(
        &self,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult>;
}
