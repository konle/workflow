use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use domain::shared::job::{ExecuteWorkflowJob, TaskDispatcher, WorkflowEvent};
use domain::workflow::{
    entity::WorkflowInstanceEntity,
    service::{WorkflowDefinitionService, WorkflowInstanceService},
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use crate::error::ApiError;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateWorkflowInstanceRequest {
    pub workflow_meta_id: String,
    pub version: u32,
    #[serde(default)]
    pub context: JsonValue,
}

#[derive(Clone)]
pub struct WorkflowInstanceHandler {
    definition_service: WorkflowDefinitionService,
    instance_service: WorkflowInstanceService,
    dispatcher: Arc<dyn TaskDispatcher>,
}

impl WorkflowInstanceHandler {
    pub fn new(
        definition_service: WorkflowDefinitionService,
        instance_service: WorkflowInstanceService,
        dispatcher: Arc<dyn TaskDispatcher>,
    ) -> Self {
        Self { definition_service, instance_service, dispatcher }
    }
}

pub fn routes(handler: Arc<WorkflowInstanceHandler>) -> Router {
    Router::new()
        .route("/", post(create_instance))
        .route("/:id", get(get_instance))
        .route("/:id/execute", post(execute_instance))
        .route("/:id/cancel", post(cancel_instance))
        .route("/:id/retry", post(retry_instance))
        .route("/:id/resume", post(resume_instance))
        .with_state(handler)
}

/// Load template, expand nodes, persist as Pending (epoch=0).
async fn create_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Json(req): Json<CreateWorkflowInstanceRequest>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let workflow_entity = handler.definition_service
        .get_workflow_entity(req.workflow_meta_id, req.version)
        .await?;

    let instance = handler.instance_service
        .create_instance(&workflow_entity, req.context, None, 0)
        .await?;

    Ok(Json(Response::success(instance)))
}

async fn get_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.instance_service.get_workflow_instance(id).await?;
    Ok(Json(Response::success(result)))
}

/// CAS Pending -> Running, then dispatch ExecuteWorkflowJob(Start) to queue.
async fn execute_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let updated = handler.instance_service.start_instance(&id).await?;

    handler.dispatcher.dispatch_workflow(ExecuteWorkflowJob {
        workflow_instance_id: updated.workflow_instance_id.clone(),
        tenant_id: String::new(),
        event: WorkflowEvent::Start,
    }).await.map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(Response::success(updated)))
}

async fn cancel_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.instance_service.cancel_instance(&id).await?;
    Ok(Json(Response::success(result)))
}

async fn retry_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.instance_service.retry_instance(&id).await?;
    Ok(Json(Response::success(result)))
}

async fn resume_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.instance_service.resume_instance(&id).await?;
    Ok(Json(Response::success(result)))
}
