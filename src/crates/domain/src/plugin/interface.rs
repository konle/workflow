use async_trait::async_trait;
use crate::shared::workflow::TaskType;
use crate::shared::job::ExecuteTaskJob;
use crate::workflow::entity::{WorkflowNodeInstanceEntity, WorkflowInstanceEntity, NodeExecutionStatus};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: NodeExecutionStatus,
    pub dispatch_jobs: Vec<ExecuteTaskJob>,
    pub jump_to_node: Option<String>, // 插件决定走向，比如if条件为真，则跳转到then_task，否则跳转到else_task
}

impl ExecutionResult {
    pub fn success(jump_to_node: Option<String>) -> Self {
        Self { status: NodeExecutionStatus::Success, dispatch_jobs: vec![], jump_to_node: jump_to_node }
    }

    pub fn failed() -> Self {
        Self { status: NodeExecutionStatus::Failed, dispatch_jobs: vec![], jump_to_node: None }
    }

    pub fn suspended() -> Self {
        Self { status: NodeExecutionStatus::Suspended, dispatch_jobs: vec![], jump_to_node: None }
    }

    pub fn pending() -> Self {
        Self { status: NodeExecutionStatus::Pending, dispatch_jobs: vec![], jump_to_node: None }
    }

    pub fn async_dispatch(job: ExecuteTaskJob) -> Self {
        Self { status: NodeExecutionStatus::Suspended, dispatch_jobs: vec![job], jump_to_node: None }
    }

    pub fn async_dispatch_multiple(jobs: Vec<ExecuteTaskJob>) -> Self {
        Self { status: NodeExecutionStatus::Suspended, dispatch_jobs: jobs, jump_to_node: None }
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
