use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use domain::shared::form::Form;
use domain::shared::workflow::WorkflowStatus;
use domain::workflow::{
    entity::{WorkflowEntity, WorkflowMetaEntity, WorkflowNodeEntity},
    service::WorkflowDefinitionService,
};
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
use crate::response::response::Response;
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateWorkflowMetaRequest {
    pub name: String,
    pub description: String,
    pub status: WorkflowStatus,
    #[serde(default)]
    pub form: Vec<Form>,
}

#[derive(Deserialize)]
pub struct UpdateWorkflowMetaRequest {
    pub name: String,
    pub description: String,
    pub status: WorkflowStatus,
    #[serde(default)]
    pub form: Vec<Form>,
}

#[derive(Deserialize)]
pub struct SaveWorkflowTemplateRequest {
    pub version: u32,
    pub status: WorkflowStatus,
    pub nodes: Vec<WorkflowNodeEntity>,
}

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
        .route("/meta", post(create_workflow_meta).get(list_workflow_meta))
        .route("/meta/{workflow_meta_id}", get(get_workflow_meta).put(update_workflow_meta).delete(delete_workflow_meta))
        .route("/meta/{workflow_meta_id}/template", post(save_workflow_template).get(list_workflow_templates))
        .route("/meta/{workflow_meta_id}/template/{version}", get(get_workflow_template).delete(delete_workflow_template))
        .with_state(handler)
}

async fn create_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateWorkflowMetaRequest>,
) -> Result<Json<Response<WorkflowMetaEntity>>, ApiError> {
    let now = Utc::now();
    let entity = WorkflowMetaEntity {
        workflow_meta_id: Uuid::new_v4().to_string(),
        tenant_id: auth.tenant_id,
        name: req.name,
        description: req.description,
        created_at: now,
        updated_at: now,
        deleted_at: None,
        status: req.status,
        form: req.form,
    };
    let result = handler.service.create_workflow_meta_entity(&entity).await?;
    Ok(Json(Response::success(result)))
}

async fn list_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Response<Vec<WorkflowMetaEntity>>>, ApiError> {
    let result = handler.service.list_workflow_meta_entities(&auth.tenant_id).await?;
    Ok(Json(Response::success(result)))
}

async fn get_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(workflow_meta_id): Path<String>,
) -> Result<Json<Response<WorkflowMetaEntity>>, ApiError> {
    let result = handler.service.get_workflow_meta_entity_scoped(&auth.tenant_id, &workflow_meta_id).await?;
    Ok(Json(Response::success(result)))
}

async fn update_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(workflow_meta_id): Path<String>,
    Json(req): Json<UpdateWorkflowMetaRequest>,
) -> Result<Json<Response<()>>, ApiError> {
    let existing = handler.service.get_workflow_meta_entity_scoped(&auth.tenant_id, &workflow_meta_id).await?;
    let entity = WorkflowMetaEntity {
        workflow_meta_id,
        tenant_id: auth.tenant_id,
        name: req.name,
        description: req.description,
        created_at: existing.created_at,
        updated_at: Utc::now(),
        deleted_at: existing.deleted_at,
        status: req.status,
        form: req.form,
    };
    handler.service.save_workflow_meta_entity(&entity).await?;
    Ok(Json(Response::success(())))
}

async fn delete_workflow_meta(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(workflow_meta_id): Path<String>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.delete_workflow_meta_entity(&auth.tenant_id, &workflow_meta_id).await?;
    Ok(Json(Response::success(())))
}

async fn save_workflow_template(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(workflow_meta_id): Path<String>,
    Json(req): Json<SaveWorkflowTemplateRequest>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.get_workflow_meta_entity_scoped(&auth.tenant_id, &workflow_meta_id).await?;
    let now = Utc::now();
    let entity = WorkflowEntity {
        workflow_meta_id,
        version: req.version,
        status: req.status,
        nodes: req.nodes,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    handler.service.save_workflow_entity(&entity).await?;
    Ok(Json(Response::success(())))
}

async fn list_workflow_templates(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(workflow_meta_id): Path<String>,
) -> Result<Json<Response<Vec<WorkflowEntity>>>, ApiError> {
    handler.service.get_workflow_meta_entity_scoped(&auth.tenant_id, &workflow_meta_id).await?;
    let result = handler.service.list_workflow_entities(&workflow_meta_id).await?;
    Ok(Json(Response::success(result)))
}

async fn get_workflow_template(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path((workflow_meta_id, version)): Path<(String, u32)>,
) -> Result<Json<Response<WorkflowEntity>>, ApiError> {
    handler.service.get_workflow_meta_entity_scoped(&auth.tenant_id, &workflow_meta_id).await?;
    let result = handler.service.get_workflow_entity(workflow_meta_id, version).await?;
    Ok(Json(Response::success(result)))
}

async fn delete_workflow_template(
    State(handler): State<Arc<WorkflowHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path((workflow_meta_id, version)): Path<(String, u32)>,
) -> Result<Json<Response<()>>, ApiError> {
    handler.service.get_workflow_meta_entity_scoped(&auth.tenant_id, &workflow_meta_id).await?;
    handler.service.delete_workflow_entity(workflow_meta_id, version).await?;
    Ok(Json(Response::success(())))
}
