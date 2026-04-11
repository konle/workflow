use std::sync::Arc;
use chrono::Utc;
use serde_json::Value as JsonValue;
use tracing::info;
use uuid::Uuid;
use crate::shared::job::WorkflowCallerContext;
use crate::shared::workflow::{TaskInstanceStatus, TaskType, WorkflowInstanceStatus, WorkflowStatus};
use crate::task::entity::TaskInstanceEntity;
use crate::workflow::entity::query::WorkflowInstanceQuery;
use crate::workflow::entity::workflow_definition::{
    NodeExecutionStatus, WorkflowEntity, WorkflowInstanceEntity,
    WorkflowMetaEntity, WorkflowNodeInstanceEntity,
};
use common::pagination::PaginatedData;
use crate::workflow::repository::{RepositoryError, WorkflowDefinitionRepository, WorkflowInstanceRepository};

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
        self.transition_status(workflow_meta_id.to_string(), version, &WorkflowStatus::Draft, &WorkflowStatus::Published).await
    }

    async fn transition_status(&self, workflow_meta_id: String, version: u32, from_status: &WorkflowStatus, to_status: &WorkflowStatus) -> Result<(), RepositoryError> {
        if !from_status.can_transition_to(to_status) {
            return Err(format!(
                "invalid workflow status transition: {} -> {}",
                from_status, to_status
            ).into());
        }
        self.repository.transition_status(workflow_meta_id, version, from_status, to_status).await
    }

    pub async fn copy_workflow_entity(&self, workflow_meta_id: &str, version: u32) -> Result<(), RepositoryError> {
        let max_version = self.repository.max_version(workflow_meta_id.to_string()).await?;
        let workflow_entity = self.get_workflow_entity(workflow_meta_id.to_string(), version).await?;
        if workflow_entity.status != WorkflowStatus::Published {
            return Err(format!(
                "cannot copy workflow template: workflow template is not published",
            ).into());
        }
        info!("max_version: {}", max_version);
        let new_workflow_entity = WorkflowEntity {
            workflow_meta_id: workflow_entity.workflow_meta_id.clone(),
            version: max_version + 1,
            status: WorkflowStatus::Draft,
            nodes: workflow_entity.nodes.clone(),
            entry_node: workflow_entity.entry_node.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };
        self.repository.save_workflow_entity(&new_workflow_entity).await
    }

    pub async fn delete_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<(), RepositoryError> {
        self.transition_status(workflow_meta_id.to_string(), version, &WorkflowStatus::Archived, &WorkflowStatus::Deleted).await
    }

    pub async fn archive_workflow_entity(&self, workflow_meta_id: &str, version: u32) -> Result<(), RepositoryError> {
        self.transition_status(workflow_meta_id.to_string(), version, &WorkflowStatus::Published, &WorkflowStatus::Archived).await
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

    pub async fn list_workflow_instances(
        &self,
        tenant_id: &str,
        query: &WorkflowInstanceQuery,
    ) -> Result<PaginatedData<WorkflowInstanceEntity>, RepositoryError> {
        self.repository.list_workflow_instances(tenant_id, query).await
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

    /// Failed -> Pending (user chooses to retry).
    /// Also resets the current (failed) node back to Pending so the
    /// execution loop will re-execute it instead of short-circuiting.
    pub async fn retry_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let mut instance = self.transfer_status(
            workflow_instance_id,
            &WorkflowInstanceStatus::Failed,
            &WorkflowInstanceStatus::Pending,
        ).await?;

        let current_node_id = instance.get_current_node();
        if let Some(node) = instance.nodes.iter_mut().find(|n| n.node_id == current_node_id) {
            if node.status == NodeExecutionStatus::Failed {
                node.status = NodeExecutionStatus::Pending;
                node.error_message = None;
                node.task_instance.output = None;
                node.task_instance.error_message = None;
            }
        }

        self.repository.save_workflow_instance(&instance).await?;
        Ok(instance)
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

    /// Skip the **current** node after `Failed` / `Suspended`: mark `Skipped`, persist `output` on the node task row,
    /// transition instance to `Pending`. Caller should then dispatch `WorkflowEvent::NodeCallback` (architecture §1.4.5).
    pub async fn skip_workflow_node(
        &self,
        tenant_id: &str,
        workflow_instance_id: &str,
        node_id: &str,
        child_task_id: Option<String>,
        output: JsonValue,
    ) -> Result<WorkflowInstanceEntity, String> {
        if !output.is_object() {
            return Err("output must be a JSON object".to_string());
        }

        let mut inst = self
            .get_workflow_instance_scoped(tenant_id, workflow_instance_id)
            .await
            .map_err(|e| e.to_string())?;

        if inst.get_current_node() != node_id {
            return Err("node_id must match current_node".to_string());
        }

        let idx = inst
            .nodes
            .iter()
            .position(|n| n.node_id == node_id)
            .ok_or_else(|| format!("node not found: {}", node_id))?;

        let node = &inst.nodes[idx];
        let is_container = matches!(
            node.node_type,
            TaskType::Parallel | TaskType::ForkJoin
        );

        if matches!(node.node_type, TaskType::SubWorkflow) {
            return Err(
                "SubWorkflow nodes cannot be skipped directly; skip the failed node inside the child workflow instance instead".to_string(),
            );
        }

        if is_container {
            let cid = child_task_id.as_deref().ok_or_else(|| {
                "child_task_id is required when skipping a Parallel/ForkJoin child task".to_string()
            })?;

            let prefix = format!("{}-{}-", workflow_instance_id, node_id);
            if !cid.starts_with(&prefix) {
                return Err(format!(
                    "child_task_id '{}' does not belong to container node '{}'",
                    cid, node_id
                ));
            }

            if !matches!(
                inst.status,
                WorkflowInstanceStatus::Failed | WorkflowInstanceStatus::Await
            ) {
                return Err(format!(
                    "workflow instance must be Failed or Await to skip container child, got {:?}",
                    inst.status
                ));
            }

            // For Failed instances (max_failures breaker tripped), transition back
            // to Await so the workflow worker can process the incoming NodeCallback.
            if inst.status == WorkflowInstanceStatus::Failed {
                inst.status = WorkflowInstanceStatus::Await;
                inst.updated_at = Utc::now();
                self.save_workflow_instance(&inst)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            return self
                .get_workflow_instance(workflow_instance_id.to_string())
                .await
                .map_err(|e| e.to_string());
        }

        // ── ordinary (atomic) node skip ──

        if !matches!(
            inst.status,
            WorkflowInstanceStatus::Failed | WorkflowInstanceStatus::Suspended
        ) {
            return Err(format!(
                "workflow instance must be Failed or Suspended, got {:?}",
                inst.status
            ));
        }

        if !matches!(
            node.status,
            NodeExecutionStatus::Failed | NodeExecutionStatus::Suspended
        ) {
            return Err(format!(
                "node must be Failed or Suspended to skip, got {:?}",
                node.status
            ));
        }

        inst.nodes[idx].status = NodeExecutionStatus::Skipped;
        inst.nodes[idx].task_instance.output = Some(output);
        inst.nodes[idx].task_instance.task_status = TaskInstanceStatus::Completed;
        inst.nodes[idx].task_instance.error_message = None;
        inst.nodes[idx].error_message = None;
        inst.nodes[idx].updated_at = Utc::now();
        inst.nodes[idx].task_instance.updated_at = Utc::now();

        if !inst
            .status
            .can_transition_to(&WorkflowInstanceStatus::Pending)
        {
            return Err(format!(
                "cannot transition workflow status {:?} to Pending",
                inst.status
            ));
        }
        inst.status = WorkflowInstanceStatus::Pending;
        inst.updated_at = Utc::now();

        self.save_workflow_instance(&inst)
            .await
            .map_err(|e| e.to_string())?;

        self.get_workflow_instance(workflow_instance_id.to_string())
            .await
            .map_err(|e| e.to_string())
    }

    // ── Sweeper helpers ─────────────────────────────────────────────

    pub async fn scan_zombie_instances(
        &self,
        limit: u32,
    ) -> Result<Vec<WorkflowInstanceEntity>, RepositoryError> {
        self.repository.scan_zombie_instances(limit).await
    }

    pub async fn force_clear_lock(
        &self,
        workflow_instance_id: &str,
        expected_epoch: u64,
    ) -> Result<(), RepositoryError> {
        self.repository
            .force_clear_lock(workflow_instance_id, expected_epoch)
            .await
    }

    /// Bypass state machine validation — used only by sweeper to force a status.
    pub async fn transfer_status_unchecked(
        &self,
        workflow_instance_id: &str,
        to: &WorkflowInstanceStatus,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let instance = self.get_workflow_instance(workflow_instance_id.to_string()).await?;
        self.repository
            .transfer_status(workflow_instance_id, &instance.status, to)
            .await
    }
}

/// `child_task_id` for [`crate::shared::job::WorkflowEvent::NodeCallback`]: match HTTP executor jobs (`{workflow}-{node}`).
pub fn node_callback_child_task_id(
    instance: &WorkflowInstanceEntity,
    node: &WorkflowNodeInstanceEntity,
) -> String {
    match node.node_type {
        TaskType::Http => format!("{}-{}", instance.workflow_instance_id, node.node_id),
        _ => node.task_instance.task_instance_id.clone(),
    }
}
