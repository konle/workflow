use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use domain::workflow::{
    entity::WorkflowInstanceEntity,
    service::WorkflowInstanceService,
};
use crate::error::ApiError;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct WorkflowInstanceHandler {
    service: WorkflowInstanceService,
}

impl WorkflowInstanceHandler {
    pub fn new(service: WorkflowInstanceService) -> Self {
        Self { service }
    }
}

pub fn routes(handler: Arc<WorkflowInstanceHandler>) -> Router {
    Router::new()
        .route("/:id", get(get_instance))
        .route("/:id/cancel", post(cancel_instance))
        .route("/:id/retry", post(retry_instance))
        .route("/:id/resume", post(resume_instance))
        .with_state(handler)
}

async fn get_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.service.get_workflow_instance(id).await?;
    Ok(Json(Response::success(result)))
}

async fn cancel_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.service.cancel_instance(&id).await?;
    Ok(Json(Response::success(result)))
}

async fn retry_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.service.retry_instance(&id).await?;
    Ok(Json(Response::success(result)))
}

async fn resume_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.service.resume_instance(&id).await?;
    Ok(Json(Response::success(result)))
}
