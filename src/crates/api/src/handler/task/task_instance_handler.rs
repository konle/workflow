use axum::{
    Json, Router, extract::{Extension, Path, Query, State}, middleware::from_fn, routing::{get, post}
};
use chrono::Utc;
use common::pagination::PaginatedData;
use tracing::{info, error};
use uuid::Uuid;
use domain::{shared::job::{ExecuteTaskJob, TaskDispatcher}, task::entity::query::TaskInstanceQuery};
use domain::shared::workflow::TaskInstanceStatus;
use domain::task::entity::task_definition::TaskInstanceEntity;
use domain::task::service::{TaskService, TaskInstanceService};
use domain::user::entity::Permission;
use crate::{error::ApiError, handler::task::task_instance_request::ListTaskInstancesRequest};
use crate::middleware::auth::AuthContext;
use crate::middleware::permission::require_permission;
use crate::response::response::Response;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateTaskInstanceRequest {
    pub task_id: String,
}

#[derive(Clone)]
pub struct TaskInstanceHandler {
    service: TaskInstanceService,
    task_service: TaskService,
    dispatcher: Arc<dyn TaskDispatcher>,
}

impl TaskInstanceHandler {
    pub fn new(service: TaskInstanceService, task_service: TaskService, dispatcher: Arc<dyn TaskDispatcher>) -> Self {
        Self { service, task_service, dispatcher }
    }
}

pub fn routes(handler: Arc<TaskInstanceHandler>) -> Router {
    let reads = Router::new()
        .route("/", get(list_task_instances))
        .route("/{id}", get(get_task_instance));

    let writes = Router::new()
        .route("/", post(create_task_instance))
        .route("/{id}/execute", post(execute_task_instance))
        .route("/{id}/retry", post(retry_task_instance))
        .route("/{id}/cancel", post(cancel_task_instance))
        .layer(from_fn(require_permission(Permission::InstanceExecute)));

    Router::new()
        .merge(reads)
        .merge(writes)
        .with_state(handler)
}

async fn create_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateTaskInstanceRequest>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let task = handler.task_service.get_task_entity_scoped(&auth.tenant_id, &req.task_id).await?;
    let now = Utc::now();
    let instance_id = Uuid::new_v4().to_string();
    let entity = TaskInstanceEntity {
        id: Uuid::new_v4().to_string(),
        tenant_id: auth.tenant_id,
        task_id: task.id.clone(),
        task_name: task.name.clone(),
        task_type: task.task_type.clone(),
        task_template: task.task_template.clone(),
        task_status: TaskInstanceStatus::Pending,
        task_instance_id: instance_id,
        created_at: now,
        updated_at: now,
        deleted_at: None,
        input: None,
        output: None,
        error_message: None,
        execution_duration: None,
        caller_context: None,
    };
    let result = handler.service.create_task_instance_entity(entity).await?;
    Ok(Json(Response::success(result)))
}

async fn list_task_instances(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Query(req): Query<ListTaskInstancesRequest>,
) -> Result<Json<Response<PaginatedData<TaskInstanceEntity>>>, ApiError> {
    let mut query = TaskInstanceQuery::from(req);
    query.tenant_id = auth.tenant_id.clone();
    info!("list_task_instances query: {:?} tenant_id: {}", query, auth.tenant_id);
    let result = handler.service.list_task_instance_entities(&query).await?;
    Ok(Json(Response::success(result)))
}

async fn get_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.service.get_task_instance_entity_scoped(&auth.tenant_id, &id).await?;
    Ok(Json(Response::success(result)))
}

async fn execute_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    handler.service.get_task_instance_entity_scoped(&auth.tenant_id, &id).await?;
    let updated = handler.service.submit_instance(&id).await?;

    handler.dispatcher.dispatch_task(ExecuteTaskJob {
        task_instance_id: updated.task_instance_id.clone(),
        tenant_id: auth.tenant_id,
        caller_context: None,
    }).await.map_err(|e| {
        error!(task_instance_id = %id, error = %e, "failed to dispatch task execution");
        ApiError::internal(e.to_string())
    })?;

    info!(task_instance_id = %id, "task execution dispatched");

    Ok(Json(Response::success(updated)))
}

async fn retry_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    handler.service.get_task_instance_entity_scoped(&auth.tenant_id, &id).await?;
    let result = handler.service.retry_instance(&id).await?;
    Ok(Json(Response::success(result)))
}

async fn cancel_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    handler.service.get_task_instance_entity_scoped(&auth.tenant_id, &id).await?;
    let result = handler.service.cancel_instance(&id).await?;
    Ok(Json(Response::success(result)))
}
