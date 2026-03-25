use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use tracing::{info, error};
use domain::shared::job::{ExecuteWorkflowJob, TaskDispatcher, WorkflowEvent};
use domain::workflow::{
    entity::WorkflowInstanceEntity,
    service::{WorkflowDefinitionService, WorkflowInstanceService},
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
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
        .route("/", post(create_instance).get(list_instances))
        .route("/{id}", get(get_instance))
        .route("/{id}/execute", post(execute_instance))
        .route("/{id}/cancel", post(cancel_instance))
        .route("/{id}/retry", post(retry_instance))
        .route("/{id}/resume", post(resume_instance))
        .with_state(handler)
}

async fn create_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateWorkflowInstanceRequest>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    handler.definition_service
        .get_workflow_meta_entity_scoped(&auth.tenant_id, &req.workflow_meta_id)
        .await?;

    let workflow_entity = handler.definition_service
        .get_workflow_entity(req.workflow_meta_id, req.version)
        .await?;

    let instance = handler.instance_service
        .create_instance(&auth.tenant_id, &workflow_entity, req.context, None, 0)
        .await?;

    info!(
        workflow_instance_id = %instance.workflow_instance_id,
        tenant_id = %auth.tenant_id,
        "workflow instance created"
    );

    Ok(Json(Response::success(instance)))
}

async fn list_instances(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Response<Vec<WorkflowInstanceEntity>>>, ApiError> {
    let result = handler.instance_service.list_workflow_instances(&auth.tenant_id).await?;
    Ok(Json(Response::success(result)))
}

async fn get_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    let result = handler.instance_service.get_workflow_instance_scoped(&auth.tenant_id, &id).await?;
    Ok(Json(Response::success(result)))
}

async fn execute_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    handler.instance_service.get_workflow_instance_scoped(&auth.tenant_id, &id).await?;
    let updated = handler.instance_service.start_instance(&id).await?;

    handler.dispatcher.dispatch_workflow(ExecuteWorkflowJob {
        workflow_instance_id: updated.workflow_instance_id.clone(),
        tenant_id: auth.tenant_id,
        event: WorkflowEvent::Start,
    }).await.map_err(|e| {
        error!(workflow_instance_id = %id, error = %e, "failed to dispatch workflow execution");
        ApiError::internal(e.to_string())
    })?;

    info!(workflow_instance_id = %id, "workflow execution dispatched");

    Ok(Json(Response::success(updated)))
}

async fn cancel_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    handler.instance_service.get_workflow_instance_scoped(&auth.tenant_id, &id).await?;
    let result = handler.instance_service.cancel_instance(&id).await?;
    info!(workflow_instance_id = %id, "workflow instance cancelled");
    Ok(Json(Response::success(result)))
}

async fn retry_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    handler.instance_service.get_workflow_instance_scoped(&auth.tenant_id, &id).await?;
    let result = handler.instance_service.retry_instance(&id).await?;
    info!(workflow_instance_id = %id, "workflow instance retried");
    Ok(Json(Response::success(result)))
}

async fn resume_instance(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    handler.instance_service.get_workflow_instance_scoped(&auth.tenant_id, &id).await?;
    let result = handler.instance_service.resume_instance(&id).await?;
    info!(workflow_instance_id = %id, "workflow instance resumed");
    Ok(Json(Response::success(result)))
}
