use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use domain::task::entity::TaskEntity;
use domain::task::service::TaskService;
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
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
        .route("/", post(create_task).get(list_tasks))
        .route("/{id}", get(get_task).put(update_task).delete(delete_task))
        .with_state(handler)
}

async fn create_task(
    State(handler): State<Arc<TaskHandler>>,
    Extension(auth): Extension<AuthContext>,
    Json(mut task): Json<TaskEntity>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    task.tenant_id = auth.tenant_id;
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
    Json(mut task): Json<TaskEntity>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    task.id = id;
    task.tenant_id = auth.tenant_id;
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
