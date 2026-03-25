use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use tracing::{info, error};
use domain::shared::job::{ExecuteTaskJob, TaskDispatcher};
use domain::task::entity::TaskInstanceEntity;
use domain::task::service::TaskInstanceService;
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskInstanceHandler {
    service: TaskInstanceService,
    dispatcher: Arc<dyn TaskDispatcher>,
}

impl TaskInstanceHandler {
    pub fn new(service: TaskInstanceService, dispatcher: Arc<dyn TaskDispatcher>) -> Self {
        Self { service, dispatcher }
    }
}

pub fn routes(handler: Arc<TaskInstanceHandler>) -> Router {
    Router::new()
        .route("/", post(create_task_instance).get(list_task_instances))
        .route("/{id}", get(get_task_instance).put(update_task_instance))
        .route("/{id}/execute", post(execute_task_instance))
        .route("/{id}/retry", post(retry_task_instance))
        .route("/{id}/cancel", post(cancel_task_instance))
        .with_state(handler)
}

async fn create_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Json(mut entity): Json<TaskInstanceEntity>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    entity.tenant_id = auth.tenant_id;
    let result = handler.service.create_task_instance_entity(entity).await?;
    Ok(Json(Response::success(result)))
}

async fn list_task_instances(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Response<Vec<TaskInstanceEntity>>>, ApiError> {
    let result = handler.service.list_task_instance_entities(&auth.tenant_id).await?;
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

async fn update_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(mut entity): Json<TaskInstanceEntity>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    entity.id = id;
    entity.tenant_id = auth.tenant_id;
    let result = handler.service.update_task_instance_entity(entity).await?;
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
