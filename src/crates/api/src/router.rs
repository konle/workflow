use axum::Router;
use std::sync::Arc;
use crate::handler::task::{TaskHandler, routes as task_routes};
use crate::handler::workflow::routes as workflow_routes;

pub fn create_router(task_handler: Arc<TaskHandler>) -> Router {
    let v1 = Router::new()
        // 每个 handler 模块自行注册路由，等同于 Gin 的 group.Register(&group)
        .nest("/task", task_routes(task_handler))
        .nest("/workflow", workflow_routes());

    Router::new().nest("/api/v1", v1)
}
