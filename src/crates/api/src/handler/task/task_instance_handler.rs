use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use domain::task::entity::TaskInstanceEntity;
use domain::task::service::TaskInstanceService;
use crate::error::ApiError;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskInstanceHandler {
    service: TaskInstanceService,
}

impl TaskInstanceHandler {
    pub fn new(service: TaskInstanceService) -> Self {
        Self { service }
    }
}

pub fn routes(handler: Arc<TaskInstanceHandler>) -> Router {
    Router::new()
        .route("/", post(create_task_instance))
        .route("/:id", get(get_task_instance))
        .route("/:id", put(update_task_instance))
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
