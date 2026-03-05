use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use domain::task::entity::{TaskEntity, TaskInstanceEntity};
use domain::task::service::{TaskService, TaskInstanceService};
use crate::error::ApiError;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskHandler {
    pub task_service: TaskService,
}

impl TaskHandler {
    pub fn new(task_service: TaskService) -> Self {
        Self { task_service }
    }

    pub async fn create_task_entity(&self, task: TaskEntity) -> Result<TaskEntity, Box<dyn std::error::Error + Send + Sync>> {
        self.task_service.create_task_entity(task).await
    }

    pub async fn get_task_entity(&self, id: String) -> Result<TaskEntity, Box<dyn std::error::Error + Send + Sync>> {
        self.task_service.get_task_entity(id).await
    }

    pub async fn update_task_entity(&self, task: TaskEntity) -> Result<TaskEntity, Box<dyn std::error::Error + Send + Sync>> {
        self.task_service.update_task_entity(task).await
    }

    pub async fn delete_task_entity(&self, id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.task_service.delete_task_entity(id).await
    }
}

#[derive(Clone)]
pub struct TaskInstanceHandler {
    pub task_instance_service: TaskInstanceService,
}

impl TaskInstanceHandler {
    pub fn new(task_instance_service: TaskInstanceService) -> Self {
        Self { task_instance_service }
    }

    pub async fn create_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, Box<dyn std::error::Error + Send + Sync>> {
        self.task_instance_service.create_task_instance_entity(task_instance_entity).await
    }

    pub async fn get_task_instance_entity(&self, id: String) -> Result<TaskInstanceEntity, Box<dyn std::error::Error + Send + Sync>> {
        self.task_instance_service.get_task_instance_entity(id).await
    }

    pub async fn update_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, Box<dyn std::error::Error + Send + Sync>> {
        self.task_instance_service.update_task_instance_entity(task_instance_entity).await
    }

}

// 注册路由到 group (等同于 Gin 的 group.POST、group.GET...)
// 返回一个 Router，调用方使用 nest("/task", routes(...)) 挂载
pub fn routes(handler: Arc<TaskHandler>) -> Router {
    Router::new()
        .route("/", post(create_task))
        .route("/:id", get(get_task))
        .route("/:id", put(update_task))
        .route("/:id", delete(delete_task))
        .with_state(handler)
}

pub fn task_instance_routes(handler: Arc<TaskInstanceHandler>) -> Router {
    Router::new()
        .route("/", post(create_task_instance))
        .route("/:id", get(get_task_instance))
        .route("/:id", put(update_task_instance))
        .with_state(handler)
}

// --- Axum 处理函数 ---

async fn create_task(
    State(handler): State<Arc<TaskHandler>>,
    Json(task): Json<TaskEntity>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let result = handler.create_task_entity(task).await?;
    Ok(Json(Response::success(result)))
}

async fn get_task(
    State(handler): State<Arc<TaskHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    let result = handler.get_task_entity(id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_task(
    State(handler): State<Arc<TaskHandler>>,
    Path(id): Path<String>,
    Json(mut task): Json<TaskEntity>,
) -> Result<Json<Response<TaskEntity>>, ApiError> {
    task.id = id;
    let result = handler.update_task_entity(task).await?;
    Ok(Json(Response::success(result)))
}

async fn delete_task(
    State(handler): State<Arc<TaskHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.delete_task_entity(id).await?;
    Ok(Json(Response::success(())))
}

async fn create_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Json(task_instance_entity): Json<TaskInstanceEntity>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.create_task_instance_entity(task_instance_entity).await?;
    Ok(Json(Response::success(result)))
}

async fn get_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    let result = handler.get_task_instance_entity(id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_task_instance(
    State(handler): State<Arc<TaskInstanceHandler>>,
    Path(id): Path<String>,
    Json(mut task_instance_entity): Json<TaskInstanceEntity>,
) -> Result<Json<Response<TaskInstanceEntity>>, ApiError> {
    task_instance_entity.id = id;
    let result = handler.update_task_instance_entity(task_instance_entity).await?;
    Ok(Json(Response::success(result)))
}

