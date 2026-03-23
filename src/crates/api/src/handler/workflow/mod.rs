pub mod workflow_handler;
pub mod workflow_instance_handler;

use axum::Router;
use std::sync::Arc;

pub use workflow_handler::WorkflowHandler;
pub use workflow_instance_handler::WorkflowInstanceHandler;

pub fn routes(
    workflow_handler: Arc<WorkflowHandler>,
    instance_handler: Arc<WorkflowInstanceHandler>,
) -> Router {
    Router::new()
        .merge(workflow_handler::routes(workflow_handler))
        .nest("/instance", workflow_instance_handler::routes(instance_handler))
}
