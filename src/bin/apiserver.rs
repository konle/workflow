use std::sync::Arc;
use tokio::net::TcpListener;

use infrastructure::mongodb::task::task_repository_impl::{TaskRepositoryImpl, TaskInstanceRepositoryImpl};
use infrastructure::mongodb::workflow::workflow_repository_impl::{WorkflowDefinitionRepositoryImpl, WorkflowInstanceRepositoryImpl};
use infrastructure::queue::consumer;
use infrastructure::queue::dispatcher::ApalisDispatcher;

use domain::task::service::{TaskService, TaskInstanceService};
use domain::workflow::service::{WorkflowDefinitionService, WorkflowInstanceService};

use api::handler::task::{TaskHandler, TaskInstanceHandler};
use api::handler::workflow::{WorkflowHandler, WorkflowInstanceHandler};
use api::router::create_router;

#[tokio::main]
async fn main() {
    println!("API server starting...");

    let mongo_url = std::env::var("MONGO_URL").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let mongo_client = mongodb::Client::with_uri_str(&mongo_url)
        .await
        .expect("failed to connect to MongoDB");

    let task_storage = consumer::create_task_storage(&redis_url).await;
    let workflow_storage = consumer::create_workflow_storage(&redis_url).await;
    let dispatcher: Arc<dyn domain::shared::job::TaskDispatcher> =
        Arc::new(ApalisDispatcher::new(task_storage, workflow_storage));

    let task_repo = Arc::new(TaskRepositoryImpl::new(mongo_client.clone()));
    let task_instance_repo = Arc::new(TaskInstanceRepositoryImpl::new(mongo_client.clone()));
    let workflow_def_repo = Arc::new(WorkflowDefinitionRepositoryImpl::new(mongo_client.clone()));
    let workflow_inst_repo = Arc::new(WorkflowInstanceRepositoryImpl::new(mongo_client.clone()));

    let task_service = TaskService::new(task_repo);
    let task_instance_service = TaskInstanceService::new(task_instance_repo);
    let workflow_def_service = WorkflowDefinitionService::new(workflow_def_repo);
    let workflow_inst_service = WorkflowInstanceService::new(workflow_inst_repo);

    let task_handler = Arc::new(TaskHandler::new(task_service));
    let task_instance_handler = Arc::new(TaskInstanceHandler::new(task_instance_service, dispatcher));
    let workflow_handler = Arc::new(WorkflowHandler::new(workflow_def_service));
    let workflow_instance_handler = Arc::new(WorkflowInstanceHandler::new(workflow_inst_service));

    let app = create_router(
        task_handler,
        task_instance_handler,
        workflow_handler,
        workflow_instance_handler,
    );

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to 0.0.0.0:3000");

    println!("API server ready at 0.0.0.0:3000");
    axum::serve(listener, app).await.expect("server error");
}
