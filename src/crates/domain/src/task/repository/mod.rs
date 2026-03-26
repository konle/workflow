use async_trait::async_trait;
use crate::shared::workflow::TaskInstanceStatus;
use crate::task::entity::{TaskEntity, TaskInstanceEntity};
use std::error::Error;

pub type RepositoryError = Box<dyn Error + Send + Sync>;

#[async_trait]
pub trait TaskEntityRepository: Send + Sync {
    async fn create_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError>;
    async fn get_task_entity(&self, id: String) -> Result<TaskEntity, RepositoryError>;
    async fn get_task_entity_scoped(&self, tenant_id: &str, id: &str) -> Result<TaskEntity, RepositoryError>;
    async fn list_task_entities(&self, tenant_id: &str) -> Result<Vec<TaskEntity>, RepositoryError>;
    async fn list_task_entities_by_type(&self, tenant_id: &str, task_type: &str) -> Result<Vec<TaskEntity>, RepositoryError>;
    async fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError>;
    async fn delete_task_entity(&self, tenant_id: &str, id: &str) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait TaskInstanceEntityRepository: Send + Sync {
    async fn create_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError>;
    async fn get_task_instance_entity(&self, id: String) -> Result<TaskInstanceEntity, RepositoryError>;
    async fn get_task_instance_entity_scoped(&self, tenant_id: &str, id: &str) -> Result<TaskInstanceEntity, RepositoryError>;
    async fn list_task_instance_entities(&self, tenant_id: &str) -> Result<Vec<TaskInstanceEntity>, RepositoryError>;
    async fn update_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError>;

    /// CAS-style atomic status transition.
    async fn transfer_status(
        &self,
        task_instance_id: &str,
        from_status: &TaskInstanceStatus,
        to_status: &TaskInstanceStatus,
    ) -> Result<TaskInstanceEntity, RepositoryError>;
}