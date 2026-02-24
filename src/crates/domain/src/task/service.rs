use crate::task::entity::TaskEntity;
use crate::task::repository::{TaskEntityRepository, RepositoryError};
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskService {
    pub task_entity_repository: Arc<dyn TaskEntityRepository>,
}

impl TaskService {
    pub fn new(task_entity_repository: Arc<dyn TaskEntityRepository>) -> Self {
        Self { task_entity_repository }
    }

    pub async fn create_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError> {
        self.task_entity_repository.create_task_entity(task_entity).await
    }

    pub async fn get_task_entity(&self, id: String) -> Result<TaskEntity, RepositoryError> {
        self.task_entity_repository.get_task_entity(id).await
    }

    pub async fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError> {
        self.task_entity_repository.update_task_entity(task_entity).await
    }
    
    pub async fn delete_task_entity(&self, id: String) -> Result<(), RepositoryError> {
        self.task_entity_repository.delete_task_entity(id).await
    }
}
