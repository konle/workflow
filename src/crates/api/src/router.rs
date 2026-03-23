use axum::{middleware, Router};
use std::sync::Arc;
use crate::handler::auth::{AuthHandler, routes as auth_routes};
use crate::handler::task::{TaskHandler, TaskInstanceHandler, routes as task_routes};
use crate::handler::tenant::{TenantHandler, routes as tenant_routes};
use crate::handler::user::{UserHandler, routes as user_routes};
use crate::handler::workflow::{WorkflowHandler, WorkflowInstanceHandler, routes as workflow_routes};
use crate::middleware::auth::auth_middleware;
use crate::middleware::permission::require_super_admin;
use domain::user::entity::Permission;
use crate::middleware::permission::require_permission;

pub fn create_router(
    auth_handler: Arc<AuthHandler>,
    tenant_handler: Arc<TenantHandler>,
    user_handler: Arc<UserHandler>,
    task_handler: Arc<TaskHandler>,
    task_instance_handler: Arc<TaskInstanceHandler>,
    workflow_handler: Arc<WorkflowHandler>,
    workflow_instance_handler: Arc<WorkflowInstanceHandler>,
) -> Router {
    let public = Router::new()
        .nest("/auth", auth_routes(auth_handler));

    let tenant_mgmt = tenant_routes(tenant_handler)
        .layer(middleware::from_fn(require_super_admin()));

    let user_mgmt = user_routes(user_handler)
        .layer(middleware::from_fn(require_permission(Permission::UserManage)));

    let protected = Router::new()
        .nest("/tenants", tenant_mgmt)
        .nest("/users", user_mgmt)
        .nest("/task", task_routes(task_handler, task_instance_handler))
        .nest("/workflow", workflow_routes(workflow_handler, workflow_instance_handler))
        .layer(middleware::from_fn(auth_middleware));

    let v1 = Router::new()
        .merge(public)
        .merge(protected);

    Router::new().nest("/api/v1", v1)
}
