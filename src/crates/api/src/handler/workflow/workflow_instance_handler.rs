use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware::from_fn,
    routing::{get, post},
};
use tracing::{info, error};
use domain::{
    shared::{job::{ExecuteWorkflowJob, TaskDispatcher, WorkflowEvent}, workflow::WorkflowStatus},
    user::entity::Permission,
    workflow::entity::query::WorkflowInstanceQuery,
};
use domain::workflow::{
    entity::workflow_definition::{NodeExecutionStatus, WorkflowInstanceEntity},
    service::{node_callback_child_task_id, WorkflowDefinitionService, WorkflowInstanceService},
};

use common::pagination::PaginatedData;

use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
use crate::middleware::permission::require_permission;
use crate::middleware::permission_guard::{PermissionLevel, RequireDraftInstanceCreate};
use crate::response::response::Response;
use crate::handler::workflow::workflow_instance_request::{CreateWorkflowInstanceRequest, SkipWorkflowNodeRequest, ListWorkflowInstancesRequest};
use std::sync::Arc;


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
    let reads = Router::new()
        .route("/", get(list_instances))
        .route("/{id}", get(get_instance));

    let writes = Router::new()
        .route("/", post(create_instance))
        .route("/{id}/execute", post(execute_instance))
        .route("/{id}/cancel", post(cancel_instance))
        .route("/{id}/retry", post(retry_instance))
        .route("/{id}/resume", post(resume_instance))
        .route("/{id}/skip-node", post(skip_node))
        .layer(from_fn(require_permission(Permission::InstanceExecute)));

    Router::new()
        .merge(reads)
        .merge(writes)
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
            RequireDraftInstanceCreate::check(&auth)?;
        }
        WorkflowStatus::Published => {}
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
    Query(req): Query<ListWorkflowInstancesRequest>,
) -> Result<Json<Response<PaginatedData<WorkflowInstanceEntity>>>, ApiError> {
    let mut query = WorkflowInstanceQuery::from(req);
    query.tenant_id = auth.tenant_id.clone();
    info!("list_instances query: {:?} tenant_id: {}", query, auth.tenant_id);
    let result = handler.instance_service.list_workflow_instances(&auth.tenant_id, &query).await?;
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

