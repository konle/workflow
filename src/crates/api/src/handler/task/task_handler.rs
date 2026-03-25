use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use domain::shared::workflow::{TaskStatus, TaskType};
use domain::task::entity::{TaskEntity, TaskTemplate};
use domain::task::service::TaskService;
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
use crate::response::response::Response;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub task_type: TaskType,
    pub task_template: TaskTemplate,
    pub description: String,
    pub status: TaskStatus,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub name: String,
    pub task_type: TaskType,
    pub task_template: TaskTemplate,
    pub description: String,
    pub status: TaskStatus,
}

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
        .route("/", post(create_task).get(list_tasks))
        .route("/{id}", get(get_task).put(update_task).delete(delete_task))
        .with_state(handler)
}

async fn create_task(
    State(handler): State<Arc<TaskHandler>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let now = Utc::now();
    let task = TaskEntity::new(
        String::new(),
        auth.tenant_id,
        req.name,
        req.task_type,
        req.task_template,
        req.description,
        req.status,
        now,
        now,
        None,
    );
    let result = handler.service.create_task_entity(task).await?;
    Ok(Json(Response::success(result)))
}

async fn list_tasks(
    State(handler): State<Arc<TaskHandler>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Response<Vec<TaskEntity>>>, ApiError> {
    let result = handler.service.list_task_entities(&auth.tenant_id).await?;
    Ok(Json(Response::success(result)))
}

async fn get_task(
    State(handler): State<Arc<TaskHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let result = handler.service.get_task_entity_scoped(&auth.tenant_id, &id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_task(
    State(handler): State<Arc<TaskHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let now = Utc::now();
    let existing = handler.service.get_task_entity_scoped(&auth.tenant_id, &id).await?;
    let task = TaskEntity::new(
        id,
        auth.tenant_id,
        req.name,
        req.task_type,
        req.task_template,
        req.description,
        req.status,
        existing.created_at,
        now,
        existing.deleted_at,
    );
    let result = handler.service.update_task_entity(task).await?;
    Ok(Json(Response::success(result)))
}

async fn delete_task(
    State(handler): State<Arc<TaskHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.delete_task_entity(&auth.tenant_id, &id).await?;
    Ok(Json(Response::success(())))
}
