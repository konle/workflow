use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use domain::shared::job::{ExecuteTaskJob, TaskDispatcher};
use domain::task::entity::TaskInstanceEntity;
use domain::task::service::TaskInstanceService;
use crate::error::ApiError;
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
        .route("/", post(create_task_instance))
        .route("/:id", get(get_task_instance))
        .route("/:id", put(update_task_instance))
        .route("/:id/execute", post(execute_task_instance))
        .route("/:id/retry", post(retry_task_instance))
        .route("/:id/cancel", post(cancel_task_instance))
        .with_state(handler)
}

async fn create_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Json(task_instance_entity): Json<TaskInstanceEntity>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.service.create_task_instance_entity(task_instance_entity).await?;
    Ok(Json(Response::success(result)))
}

async fn get_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.service.get_task_instance_entity(id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
    Json(mut task_instance_entity): Json<TaskInstanceEntity>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    task_instance_entity.id = id;
    let result = handler.service.update_task_instance_entity(task_instance_entity).await?;
    Ok(Json(Response::success(result)))
}

/// Pending -> Running, then dispatch to task worker queue.
async fn execute_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let updated = handler.service.submit_instance(&id).await?;

    handler.dispatcher.dispatch_task(ExecuteTaskJob {
        task_instance_id: updated.task_instance_id.clone(),
        tenant_id: String::new(),
        caller_context: None,
    }).await.map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(Response::success(updated)))
}

/// Failed -> Pending, ready for re-execution.
async fn retry_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.service.retry_instance(&id).await?;
    Ok(Json(Response::success(result)))
}

/// Pending | Failed -> Canceled.
async fn cancel_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.service.cancel_instance(&id).await?;
    Ok(Json(Response::success(result)))
}
