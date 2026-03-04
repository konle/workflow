use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteWorkflowJob {
    pub workflow_instance_id: String,
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCallerContext {
    pub workflow_instance_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteTaskJob {
    pub task_instance_id: String,
    pub tenant_id: String,
    pub caller_context: Option<WorkflowCallerContext>,
}

#[async_trait]
pub trait TaskDispatcher: Send + Sync {
    async fn dispatch_task(&self, job: ExecuteTaskJob) -> anyhow::Result<()>;
    async fn dispatch_workflow(&self, job: ExecuteWorkflowJob) -> anyhow::Result<()>;
}
