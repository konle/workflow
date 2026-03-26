use std::sync::Arc;
use chrono::Utc;
use serde_json::Value as JsonValue;
use uuid::Uuid;
use crate::shared::job::WorkflowCallerContext;
use crate::shared::workflow::{TaskInstanceStatus, WorkflowInstanceStatus};
use crate::task::entity::TaskInstanceEntity;
use crate::workflow::entity::{
    NodeExecutionStatus, WorkflowEntity, WorkflowInstanceEntity,
    WorkflowMetaEntity, WorkflowNodeInstanceEntity,
};
use crate::workflow::repository::{WorkflowDefinitionRepository, WorkflowInstanceRepository, RepositoryError};

#[derive(Clone)]
pub struct WorkflowDefinitionService {
    pub repository: Arc<dyn WorkflowDefinitionRepository>,
}

impl WorkflowDefinitionService {
    pub fn new(repository: Arc<dyn WorkflowDefinitionRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_workflow_meta_entity(&self, workflow_meta_entity: &WorkflowMetaEntity) -> Result<WorkflowMetaEntity, RepositoryError> {
        self.repository.create_workflow_meta_entity(workflow_meta_entity).await
    }

    pub async fn get_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<WorkflowEntity, RepositoryError> {
        self.repository.get_workflow_entity(workflow_meta_id, version).await
    }

    pub async fn list_workflow_entities(&self, workflow_meta_id: &str) -> Result<Vec<WorkflowEntity>, RepositoryError> {
        self.repository.list_workflow_entities(workflow_meta_id).await
    }

    pub async fn save_workflow_entity(&self, entity: &WorkflowEntity) -> Result<(), RepositoryError> {
        self.repository.save_workflow_entity(entity).await
    }

    pub async fn publish_workflow_entity(&self, workflow_meta_id: &str, version: u32) -> Result<(), RepositoryError> {
        self.repository.publish_workflow_entity(workflow_meta_id, version).await
    }

    pub async fn delete_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<(), RepositoryError> {
        self.repository.delete_workflow_entity(workflow_meta_id, version).await
    }

    pub async fn get_workflow_meta_entity(&self, workflow_meta_id: String) -> Result<WorkflowMetaEntity, RepositoryError> {
        self.repository.get_workflow_meta_entity(workflow_meta_id).await
    }

    pub async fn get_workflow_meta_entity_scoped(&self, tenant_id: &str, workflow_meta_id: &str) -> Result<WorkflowMetaEntity, RepositoryError> {
        self.repository.get_workflow_meta_entity_scoped(tenant_id, workflow_meta_id).await
    }

    pub async fn list_workflow_meta_entities(&self, tenant_id: &str) -> Result<Vec<WorkflowMetaEntity>, RepositoryError> {
        self.repository.list_workflow_meta_entities(tenant_id).await
    }

    pub async fn save_workflow_meta_entity(&self, entity: &WorkflowMetaEntity) -> Result<(), RepositoryError> {
        self.repository.save_workflow_meta_entity(entity).await
    }

    pub async fn delete_workflow_meta_entity(&self, tenant_id: &str, workflow_meta_id: &str) -> Result<(), RepositoryError> {
        self.repository.delete_workflow_meta_entity(tenant_id, workflow_meta_id).await
    }
}

#[derive(Clone)]
pub struct WorkflowInstanceService {
    pub repository: Arc<dyn WorkflowInstanceRepository>,
}

impl WorkflowInstanceService {
    pub fn new(repository: Arc<dyn WorkflowInstanceRepository>) -> Self {
        Self { repository }
    }

    pub async fn get_workflow_instance(&self, id: String) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.repository.get_workflow_instance(id).await
    }

    pub async fn get_workflow_instance_scoped(&self, tenant_id: &str, id: &str) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.repository.get_workflow_instance_scoped(tenant_id, id).await
    }

    pub async fn list_workflow_instances(&self, tenant_id: &str) -> Result<Vec<WorkflowInstanceEntity>, RepositoryError> {
        self.repository.list_workflow_instances(tenant_id).await
    }

    /// Expand a workflow template into a runnable instance (Pending, epoch=0).
    pub async fn create_instance(
        &self,
        tenant_id: &str,
        workflow_entity: &WorkflowEntity,
        context: JsonValue,
        parent_context: Option<WorkflowCallerContext>,
        depth: u32,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let now = Utc::now();
        let instance_id = Uuid::new_v4().to_string();

        let entry_node = workflow_entity.entry_node.clone();

        let nodes: Vec<WorkflowNodeInstanceEntity> = workflow_entity.nodes.iter().map(|node| {
            let task_instance_id = Uuid::new_v4().to_string();
            WorkflowNodeInstanceEntity {
                node_id: node.node_id.clone(),
                node_type: node.node_type.clone(),
                task_instance: TaskInstanceEntity {
                    id: task_instance_id.clone(),
                    tenant_id: tenant_id.to_string(),
                    task_id: node.task_id.clone().unwrap_or_default(),
                    task_name: String::from(""),
                    task_type: node.node_type.clone(),
                    task_template: node.config.clone(),
                    task_status: TaskInstanceStatus::Pending,
                    task_instance_id,
                    created_at: now,
                    updated_at: now,
                    deleted_at: None,
                    input: None,
                    output: None,
                    error_message: None,
                    execution_duration: None,
                    caller_context: None,
                },
                context: node.context.clone(),
                next_node: node.next_node.clone(),
                status: NodeExecutionStatus::Pending,
                output: None,
                error_message: None,
                created_at: now,
                updated_at: now,
            }
        }).collect();

        let instance = WorkflowInstanceEntity {
            workflow_instance_id: instance_id,
            tenant_id: tenant_id.to_string(),
            workflow_meta_id: workflow_entity.workflow_meta_id.clone(),
            workflow_version: workflow_entity.version,
            status: WorkflowInstanceStatus::Pending,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            context,
            entry_node: entry_node.clone(),
            current_node: entry_node,
            nodes,
            epoch: 0,
            locked_by: None,
            locked_duration: None,
            locked_at: None,
            parent_context,
            depth,
        };

        self.repository.create_workflow_instance(&instance).await
    }

    pub async fn acquire_lock(&self, workflow_instance_id: &str, worker_id: &str, duration_ms: u64) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.repository.acquire_lock(workflow_instance_id, worker_id, duration_ms).await
    }

    pub async fn release_lock(&self, workflow_instance_id: &str, worker_id: &str) -> Result<(), RepositoryError> {
        self.repository.release_lock(workflow_instance_id, worker_id).await
    }

    pub async fn save_workflow_instance(&self, instance: &WorkflowInstanceEntity) -> Result<(), RepositoryError> {
        // CAS is handled inside the repository by checking the epoch.
        // We do not increment the epoch here, the repository should do that during the update
        // to ensure it accurately reflects the DB state.
        self.repository.save_workflow_instance(instance).await
    }

    /// Core state transfer: validates the transition against the state machine,
    /// then delegates to the repository for CAS update.
    async fn transfer_status(
        &self,
        workflow_instance_id: &str,
        from: &WorkflowInstanceStatus,
        to: &WorkflowInstanceStatus,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        if !from.can_transition_to(to) {
            return Err(format!(
                "invalid state transition: {} -> {}",
                from, to
            ).into());
        }
        self.repository
            .transfer_status(workflow_instance_id, from, to)
            .await
    }

    /// Pending -> Running
    pub async fn start_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Pending,
            &WorkflowInstanceStatus::Running,
        ).await
    }

    /// Running -> Completed
    pub async fn complete_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Running,
            &WorkflowInstanceStatus::Completed,
        ).await
    }

    /// Running -> Failed
    pub async fn fail_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Running,
            &WorkflowInstanceStatus::Failed,
        ).await
    }

    /// Running -> Suspended (e.g. approval node awaiting external action)
    pub async fn suspend_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Running,
            &WorkflowInstanceStatus::Suspended,
        ).await
    }

    /// Running -> Await (yield CPU and wait for async callback)
    pub async fn await_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Running,
            &WorkflowInstanceStatus::Await,
        ).await
    }

    /// Failed -> Pending (user chooses to retry)
    pub async fn retry_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Failed,
            &WorkflowInstanceStatus::Pending,
        ).await
    }

    /// Suspended -> Pending (user approves / chooses to continue)
    pub async fn resume_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Suspended,
            &WorkflowInstanceStatus::Pending,
        ).await
    }

    /// Await -> Pending (callback received, ready to be scheduled again)
    pub async fn wake_from_await(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Await,
            &WorkflowInstanceStatus::Pending,
        ).await
    }

    /// Failed | Suspended -> Canceled (user gives up)
    pub async fn cancel_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let instance = self.get_workflow_instance(workflow_instance_id.to_string()).await?;

        if instance.status.is_terminal() {
            return Err(format!(
                "cannot cancel instance in terminal state: {}",
                instance.status
            ).into());
        }

        match instance.status {
            WorkflowInstanceStatus::Failed | WorkflowInstanceStatus::Suspended => {
                self.transfer_status(
                    workflow_instance_id,
                    &instance.status,
                    &WorkflowInstanceStatus::Canceled,
                ).await
            }
            other => Err(format!(
                "cannot cancel instance in state: {}, only Failed or Suspended can be canceled",
                other
            ).into()),
        }
    }
}
