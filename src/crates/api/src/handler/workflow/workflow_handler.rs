use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use domain::workflow::{
    entity::{WorkflowEntity, WorkflowMetaEntity},
    service::WorkflowDefinitionService,
};
use crate::error::ApiError;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct WorkflowHandler {
    service: WorkflowDefinitionService,
}

impl WorkflowHandler {
    pub fn new(service: WorkflowDefinitionService) -> Self {
        Self { service }
    }
}

pub fn routes(handler: Arc<WorkflowHandler>) -> Router {
    Router::new()
        .route("/meta", post(create_workflow_meta))
        .route("/meta/:workflow_meta_id", get(get_workflow_meta))
        .route("/meta/:workflow_meta_id", put(update_workflow_meta))
        .route("/meta/:workflow_meta_id", delete(delete_workflow_meta))
        .route("/meta/:workflow_meta_id/template", post(save_workflow_template))
        .route("/meta/:workflow_meta_id/template/:version", get(get_workflow_template))
        .route("/meta/:workflow_meta_id/template/:version", delete(delete_workflow_template))
        .with_state(handler)
}

async fn create_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Json(entity): Json<WorkflowMetaEntity>,
) -> Result<Json<Response<WorkflowMetaEntity>>, ApiError> {
    let result = handler.service.create_workflow_meta_entity(&entity).await?;
    Ok(Json(Response::success(result)))
}

async fn get_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Path(workflow_meta_id): Path<String>,
) -> Result<Json<Response<WorkflowMetaEntity>>, ApiError> {
    let result = handler.service.get_workflow_meta_entity(workflow_meta_id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Path(workflow_meta_id): Path<String>,
    Json(mut entity): Json<WorkflowMetaEntity>,
) -> Result<Json<Response<()>>, ApiError> {
    entity.workflow_meta_id = workflow_meta_id;
    handler.service.save_workflow_meta_entity(&entity).await?;
    Ok(Json(Response::success(())))
}

async fn delete_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Path(workflow_meta_id): Path<String>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.delete_workflow_meta_entity(workflow_meta_id).await?;
    Ok(Json(Response::success(())))
}

async fn save_workflow_template(
    State(handler): State<Arc<WorkflowHandler>>,
    Path(workflow_meta_id): Path<String>,
    Json(mut entity): Json<WorkflowEntity>,
) -> Result<Json<Response<()>>, ApiError> {
    entity.workflow_meta_id = workflow_meta_id;
    handler.service.save_workflow_entity(&entity).await?;
    Ok(Json(Response::success(())))
}

async fn get_workflow_template(
    State(handler): State<Arc<WorkflowHandler>>,
    Path((workflow_meta_id, version)): Path<(String, u32)>,
) -> Result<Json<Response<WorkflowEntity>>, ApiError> {
    let result = handler.service.get_workflow_entity(workflow_meta_id, version).await?;
    Ok(Json(Response::success(result)))
}

async fn delete_workflow_template(
    State(handler): State<Arc<WorkflowHandler>>,
    Path((workflow_meta_id, version)): Path<(String, u32)>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.delete_workflow_entity(workflow_meta_id, version).await?;
    Ok(Json(Response::success(())))
}
