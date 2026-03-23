pub mod workflow_handler;
pub mod workflow_instance_handler;

use axum::Router;
use std::sync::Arc;
use crate::handler::variable::{VariableHandler, workflow_meta_variable_routes};

pub use workflow_handler::WorkflowHandler;
pub use workflow_instance_handler::WorkflowInstanceHandler;

pub fn routes(
    workflow_handler: Arc<WorkflowHandler>,
    instance_handler: Arc<WorkflowInstanceHandler>,
    variable_handler: Arc<VariableHandler>,
) -> Router {
    Router::new()
        .merge(workflow_handler::routes(workflow_handler))
        .nest("/instance", workflow_instance_handler::routes(instance_handler))
        .nest("/meta/{meta_id}/variables", workflow_meta_variable_routes(variable_handler))
}
