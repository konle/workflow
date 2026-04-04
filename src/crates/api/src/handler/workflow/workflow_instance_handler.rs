use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use tracing::{info, error};
use domain::{shared::{job::{ExecuteWorkflowJob, TaskDispatcher, WorkflowEvent}, workflow::WorkflowStatus}, user::entity::TenantRole};
use domain::workflow::{
    entity::{NodeExecutionStatus, WorkflowInstanceEntity},
    service::{node_callback_child_task_id, WorkflowDefinitionService, WorkflowInstanceService},
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
use crate::response::response::Response;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct SkipWorkflowNodeRequest {
    pub node_id: String,
    pub output: JsonValue,
}

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
        // .route("/test", post(create_draft_instance)) // for test，租户开发者和管理员可以对草稿发起运行开发期间的测试
        .route("/{id}", get(get_instance))
        .route("/{id}/execute", post(execute_instance))
        .route("/{id}/cancel", post(cancel_instance))
        .route("/{id}/retry", post(retry_instance))
        .route("/{id}/resume", post(resume_instance))
        .route("/{id}/skip-node", post(skip_node))
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

    match workflow_entity.status {
        WorkflowStatus::Draft => {
            if auth.role != Some(TenantRole::Developer) && auth.role != Some(TenantRole::TenantAdmin) {
                return Err(ApiError::bad_request("only developer and tenant admin can create draft instance"));
            }
        }
        WorkflowStatus::Published => {
            
        }
        _ => {
            return Err(ApiError::bad_request("workflow is not a draft or published"));
        }
    }

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

async fn skip_node(
    State(handler): State<Arc<WorkflowInstanceHandler>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<SkipWorkflowNodeRequest>,
) -> Result<Json<Response<WorkflowInstanceEntity>>, ApiError> {
    handler
        .instance_service
        .get_workflow_instance_scoped(&auth.tenant_id, &id)
        .await?;

    let updated = handler
        .instance_service
        .skip_workflow_node(&auth.tenant_id, &id, &req.node_id, req.output.clone())
        .await
        .map_err(ApiError::bad_request)?;

    let node = updated
        .nodes
        .iter()
        .find(|n| n.node_id == req.node_id)
        .ok_or_else(|| ApiError::internal("skipped node missing after save"))?;

    let out = node.task_instance.output.clone().unwrap_or(req.output);

    handler
        .dispatcher
        .dispatch_workflow(ExecuteWorkflowJob {
            workflow_instance_id: updated.workflow_instance_id.clone(),
            tenant_id: auth.tenant_id.clone(),
            event: WorkflowEvent::NodeCallback {
                node_id: req.node_id.clone(),
                child_task_id: node_callback_child_task_id(&updated, node),
                status: NodeExecutionStatus::Skipped,
                output: Some(out),
                error_message: None,
                input: None,
            },
        })
        .await
        .map_err(|e| {
            error!(workflow_instance_id = %id, error = %e, "failed to dispatch skip NodeCallback");
            ApiError::internal(e.to_string())
        })?;

    info!(workflow_instance_id = %id, node_id = %req.node_id, "skip-node persisted and NodeCallback dispatched");

    Ok(Json(Response::success(updated)))
}

