use axum::Router;
use std::sync::Arc;
use crate::handler::task::{TaskHandler, TaskInstanceHandler, routes as task_routes};
use crate::handler::workflow::{WorkflowHandler, WorkflowInstanceHandler, routes as workflow_routes};

pub fn create_router(
    task_handler: Arc<TaskHandler>,
    task_instance_handler: Arc<TaskInstanceHandler>,
    workflow_handler: Arc<WorkflowHandler>,
    workflow_instance_handler: Arc<WorkflowInstanceHandler>,
) -> Router {
    let v1 = Router::new()
        .nest("/task", task_routes(task_handler, task_instance_handler))
        .nest("/workflow", workflow_routes(workflow_handler, workflow_instance_handler));

    Router::new().nest("/api/v1", v1)
}
