use async_trait::async_trait;
use std::error::Error;
use crate::shared::workflow::WorkflowInstanceStatus;
use crate::workflow::entity::{WorkflowEntity, WorkflowInstanceEntity};
use crate::workflow::entity::WorkflowMetaEntity;
pub type RepositoryError = Box<dyn Error + Send + Sync>;

#[async_trait]
pub trait WorkflowDefinitionRepository: Send + Sync {
    async fn get_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<WorkflowEntity, RepositoryError>;
    async fn save_workflow_entity(&self, entity: &WorkflowEntity) -> Result<(), RepositoryError>;
    async fn delete_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<(), RepositoryError>;
    // 元数据表接口定义
    async fn get_workflow_meta_entity(&self, workflow_meta_id: String) -> Result<WorkflowMetaEntity, RepositoryError>;
    async fn save_workflow_meta_entity(&self, entity: &WorkflowMetaEntity) -> Result<(), RepositoryError>;
    async fn delete_workflow_meta_entity(&self, workflow_meta_id: String) -> Result<(), RepositoryError>;
    async fn create_workflow_meta_entity(&self, workflow_meta_entity: &WorkflowMetaEntity) -> Result<WorkflowMetaEntity, RepositoryError>;
}

#[async_trait]
pub trait WorkflowInstanceRepository: Send + Sync {
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

    async fn acquire_lock(
        &self,
        workflow_instance_id: &str,
        worker_id: &str,
        duration_ms: u64,
    ) -> Result<WorkflowInstanceEntity, RepositoryError>;

    async fn release_lock(
        &self,
        workflow_instance_id: &str,
        worker_id: &str,
    ) -> Result<(), RepositoryError>;

    async fn save_workflow_instance(
        &self,
        instance: &WorkflowInstanceEntity,
    ) -> Result<(), RepositoryError>;
}
