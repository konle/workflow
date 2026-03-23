use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use domain::task::entity::TaskEntity;
use domain::task::service::TaskService;
use crate::error::ApiError;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskHandler {
    service: TaskService,
}

impl TaskHandler {
    pub fn new(service: TaskService) -> Self {
        Self { service }
    }
}

pub fn routes(handler: Arc<TaskHandler>) -> Router {
    Router::new()
        .route("/", post(create_task))
        .route("/:id", get(get_task))
        .route("/:id", put(update_task))
        .route("/:id", delete(delete_task))
        .with_state(handler)
}

async fn create_task(
    State(handler): State<Arc<TaskHandler>>,
    Json(task): Json<TaskEntity>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let result = handler.service.create_task_entity(task).await?;
    Ok(Json(Response::success(result)))
}

async fn get_task(
    State(handler): State<Arc<TaskHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let result = handler.service.get_task_entity(id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_task(
    State(handler): State<Arc<TaskHandler>>,
    Path(id): Path<String>,
    Json(mut task): Json<TaskEntity>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    task.id = id;
    let result = handler.service.update_task_entity(task).await?;
    Ok(Json(Response::success(result)))
}

async fn delete_task(
    State(handler): State<Arc<TaskHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.delete_task_entity(id).await?;
    Ok(Json(Response::success(())))
}
