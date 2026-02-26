use async_trait::async_trait;
use std::error::Error;
use crate::shared::workflow::WorkflowInstanceStatus;
use crate::workflow::entity::{WorkflowEntity, WorkflowInstanceEntity};

pub type RepositoryError = Box<dyn Error + Send + Sync>;

#[async_trait]
pub trait WorkflowEntityRepository: Send + Sync {
    async fn get_workflow_entity(&self, id: String) -> Result<WorkflowEntity, RepositoryError>;
    async fn get_workflow_instance(&self, id: String) -> Result<WorkflowInstanceEntity, RepositoryError>;

    /// CAS-style status update: only succeeds if the current status in DB matches `from_status`.
    /// Uses filter(workflow_instance_id, status=from_status) to atomically update to `to_status`.
    /// Returns the updated entity on success, or an error if the precondition fails.
    async fn transfer_status(
        &self,
        workflow_instance_id: &str,
        from_status: &WorkflowInstanceStatus,
        to_status: &WorkflowInstanceStatus,
    ) -> Result<WorkflowInstanceEntity, RepositoryError>;

    async fn save_workflow_instance(
        &self,
        instance: &WorkflowInstanceEntity,
    ) -> Result<(), RepositoryError>;
}
