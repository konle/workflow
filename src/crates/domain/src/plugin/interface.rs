use async_trait::async_trait;
use crate::shared::workflow::TaskType;
use crate::shared::job::ExecuteTaskJob;
use crate::workflow::entity::{WorkflowNodeInstanceEntity, WorkflowInstanceEntity, NodeExecutionStatus};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: NodeExecutionStatus,
    pub dispatch_job: Option<ExecuteTaskJob>,
}

impl ExecutionResult {
    pub fn success() -> Self {
        Self { status: NodeExecutionStatus::Success, dispatch_job: None }
    }

    pub fn failed() -> Self {
        Self { status: NodeExecutionStatus::Failed, dispatch_job: None }
    }

    pub fn suspended() -> Self {
        Self { status: NodeExecutionStatus::Suspended, dispatch_job: None }
    }

    pub fn pending() -> Self {
        Self { status: NodeExecutionStatus::Pending, dispatch_job: None }
    }

    pub fn async_dispatch(job: ExecuteTaskJob) -> Self {
        Self { status: NodeExecutionStatus::Suspended, dispatch_job: Some(job) }
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
