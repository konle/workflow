use crate::shared::workflow::TaskInstanceStatus;
use crate::task::entity::{TaskEntity, TaskInstanceEntity};
use crate::task::repository::{TaskEntityRepository, TaskInstanceEntityRepository, RepositoryError};
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

    pub async fn get_task_entity_scoped(&self, tenant_id: &str, id: &str) -> Result<TaskEntity, RepositoryError> {
        self.task_entity_repository.get_task_entity_scoped(tenant_id, id).await
    }

    pub async fn list_task_entities(&self, tenant_id: &str) -> Result<Vec<TaskEntity>, RepositoryError> {
        self.task_entity_repository.list_task_entities(tenant_id).await
    }

    pub async fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError> {
        self.task_entity_repository.update_task_entity(task_entity).await
    }

    pub async fn delete_task_entity(&self, tenant_id: &str, id: &str) -> Result<(), RepositoryError> {
        self.task_entity_repository.delete_task_entity(tenant_id, id).await
    }
}

#[derive(Clone)]
pub struct TaskInstanceService {
    pub task_instance_entity_repository: Arc<dyn TaskInstanceEntityRepository>,
}

impl TaskInstanceService {
    pub fn new(task_instance_entity_repository: Arc<dyn TaskInstanceEntityRepository>) -> Self {
        Self { task_instance_entity_repository }
    }

    pub async fn create_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError> {
        self.task_instance_entity_repository.create_task_instance_entity(task_instance_entity).await
    }

    pub async fn get_task_instance_entity(&self, id: String) -> Result<TaskInstanceEntity, RepositoryError> {
        self.task_instance_entity_repository.get_task_instance_entity(id).await
    }

    pub async fn get_task_instance_entity_scoped(&self, tenant_id: &str, id: &str) -> Result<TaskInstanceEntity, RepositoryError> {
        self.task_instance_entity_repository.get_task_instance_entity_scoped(tenant_id, id).await
    }

    pub async fn list_task_instance_entities(&self, tenant_id: &str) -> Result<Vec<TaskInstanceEntity>, RepositoryError> {
        self.task_instance_entity_repository.list_task_instance_entities(tenant_id).await
    }

    pub async fn update_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError> {
        self.task_instance_entity_repository.update_task_instance_entity(task_instance_entity).await
    }

    async fn transfer_status(
        &self,
        task_instance_id: &str,
        from: &TaskInstanceStatus,
        to: &TaskInstanceStatus,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        if !from.can_transition_to(to) {
            return Err(format!(
                "invalid task instance state transition: {} -> {}",
                from, to
            ).into());
        }
        self.task_instance_entity_repository
            .transfer_status(task_instance_id, from, to)
            .await
    }

    /// Pending -> Running
    pub async fn submit_instance(
        &self,
        task_instance_id: &str,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        self.transfer_status(
            task_instance_id,
            &TaskInstanceStatus::Pending,
            &TaskInstanceStatus::Running,
        ).await
    }

    /// Running -> Completed
    pub async fn complete_instance(
        &self,
        task_instance_id: &str,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        self.transfer_status(
            task_instance_id,
            &TaskInstanceStatus::Running,
            &TaskInstanceStatus::Completed,
        ).await
    }

    /// Running -> Failed
    pub async fn fail_instance(
        &self,
        task_instance_id: &str,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        self.transfer_status(
            task_instance_id,
            &TaskInstanceStatus::Running,
            &TaskInstanceStatus::Failed,
        ).await
    }

    /// Failed -> Pending
    pub async fn retry_instance(
        &self,
        task_instance_id: &str,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        self.transfer_status(
            task_instance_id,
            &TaskInstanceStatus::Failed,
            &TaskInstanceStatus::Pending,
        ).await
    }

    /// Pending | Failed -> Canceled
    pub async fn cancel_instance(
        &self,
        task_instance_id: &str,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        let instance = self.get_task_instance_entity(task_instance_id.to_string()).await?;

        if instance.task_status.is_terminal() {
            return Err(format!(
                "cannot cancel task instance in terminal state: {}",
                instance.task_status
            ).into());
        }

        match instance.task_status {
            TaskInstanceStatus::Pending | TaskInstanceStatus::Failed => {
                self.transfer_status(
                    task_instance_id,
                    &instance.task_status,
                    &TaskInstanceStatus::Canceled,
                ).await
            }
            other => Err(format!(
                "cannot cancel task instance in state: {}, only Pending or Failed can be canceled",
                other
            ).into()),
        }
    }
}