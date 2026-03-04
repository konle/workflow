use async_trait::async_trait;
use crate::task::entity::{TaskEntity, TaskInstanceEntity};
use std::error::Error;

pub type RepositoryError = Box<dyn Error + Send + Sync>;

#[async_trait]
pub trait TaskEntityRepository: Send + Sync {
    async fn create_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError>;
    async fn get_task_entity(&self, id: String) -> Result<TaskEntity, RepositoryError>;
    async fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError>;
    async fn delete_task_entity(&self, id: String) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait TaskInstanceEntityRepository: Send + Sync {
    async fn create_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError>;
    async fn get_task_instance_entity(&self, id: String) -> Result<TaskInstanceEntity, RepositoryError>;
    async fn update_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError>;
}