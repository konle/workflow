use axum::{
    extract::{Extension, Path, State},
    routing::{delete, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::error::ApiError;
use crate::middleware::auth::AuthContext;
use crate::response::response::Response;
use domain::user::entity::{TenantRole, UserTenantRole};
use domain::user::service::UserService;

#[derive(Deserialize)]
pub struct AssignRoleRequest {
    pub user_id: String,
    pub role: String,
}

#[derive(Clone)]
pub struct UserHandler {
    pub service: UserService,
}

impl UserHandler {
    pub fn new(service: UserService) -> Self {
        Self { service }
    }
}

pub fn routes(handler: Arc<UserHandler>) -> Router {
    Router::new()
        .route("/", post(assign_role).get(list_users))
        .route("/{user_id}", delete(remove_role).put(update_role))
        .with_state(handler)
}

async fn list_users(
    State(handler): State<Arc<UserHandler>>,
    Extension(ctx): Extension<AuthContext>,
) -> Result<Json<Response<Vec<UserTenantRole>>>, ApiError> {
    let list = handler.service.list_tenant_users(&ctx.tenant_id).await?;
    Ok(Json(Response::success(list)))
}

async fn assign_role(
    State(handler): State<Arc<UserHandler>>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AssignRoleRequest>,
) -> Result<Json<Response<UserTenantRole>>, ApiError> {
    let role = TenantRole::from_str(&req.role)
        .ok_or_else(|| ApiError::bad_request(format!("Invalid role: {}", req.role)))?;

    let result = handler
        .service
        .assign_role(&req.user_id, &ctx.tenant_id, &role)
        .await?;
    Ok(Json(Response::success(result)))
}

async fn update_role(
    State(handler): State<Arc<UserHandler>>,
    Extension(ctx): Extension<AuthContext>,
    Path(user_id): Path<String>,
    Json(req): Json<AssignRoleRequest>,
) -> Result<Json<Response<UserTenantRole>>, ApiError> {
    let role = TenantRole::from_str(&req.role)
        .ok_or_else(|| ApiError::bad_request(format!("Invalid role: {}", req.role)))?;

    let result = handler
        .service
        .assign_role(&user_id, &ctx.tenant_id, &role)
        .await?;
    Ok(Json(Response::success(result)))
}

async fn remove_role(
    State(handler): State<Arc<UserHandler>>,
    Extension(ctx): Extension<AuthContext>,
    Path(user_id): Path<String>,
) -> Result<Json<Response<()>>, ApiError> {
    handler
        .service
        .remove_role(&user_id, &ctx.tenant_id)
        .await?;
    Ok(Json(Response::success(())))
}
