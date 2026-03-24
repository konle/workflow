use std::sync::Arc;
use tokio::net::TcpListener;

use infrastructure::mongodb::task::task_repository_impl::{TaskRepositoryImpl, TaskInstanceRepositoryImpl};
use infrastructure::mongodb::tenant::tenant_repository_impl::TenantRepositoryImpl;
use infrastructure::mongodb::user::user_repository_impl::{UserRepositoryImpl, UserTenantRoleRepositoryImpl};
use infrastructure::mongodb::approval::approval_repository_impl::ApprovalRepositoryImpl;
use infrastructure::mongodb::variable::variable_repository_impl::VariableRepositoryImpl;
use infrastructure::mongodb::workflow::workflow_repository_impl::{WorkflowDefinitionRepositoryImpl, WorkflowInstanceRepositoryImpl};
use infrastructure::queue::consumer;
use infrastructure::queue::dispatcher::ApalisDispatcher;

use domain::task::service::{TaskService, TaskInstanceService};
use domain::tenant::service::TenantService;
use domain::user::service::UserService;
use domain::approval::service::ApprovalService;
use domain::variable::service::VariableService;
use domain::workflow::service::{WorkflowDefinitionService, WorkflowInstanceService};

use api::handler::approval::ApprovalHandler;
use api::handler::auth::AuthHandler;
use api::handler::task::{TaskHandler, TaskInstanceHandler};
use api::handler::tenant::TenantHandler;
use api::handler::user::UserHandler;
use api::handler::variable::VariableHandler;
use api::handler::workflow::{WorkflowHandler, WorkflowInstanceHandler};
use api::router::create_router;

#[tokio::main]
async fn main() {
    println!("API server starting...");

    let mongo_url = std::env::var("MONGO_URL").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let encrypt_key = std::env::var("VARIABLE_ENCRYPT_KEY")
        .unwrap_or_else(|_| "workflow-default-encrypt-key-change-me".to_string());

    let mongo_client = mongodb::Client::with_uri_str(&mongo_url)
        .await
        .expect("failed to connect to MongoDB");

    let task_storage = consumer::create_task_storage(&redis_url).await;
    let workflow_storage = consumer::create_workflow_storage(&redis_url).await;
    let dispatcher: Arc<dyn domain::shared::job::TaskDispatcher> =
        Arc::new(ApalisDispatcher::new(task_storage, workflow_storage));

    let task_repo = Arc::new(TaskRepositoryImpl::new(mongo_client.clone()));
    let task_instance_repo = Arc::new(TaskInstanceRepositoryImpl::new(mongo_client.clone()));
    let tenant_repo = Arc::new(TenantRepositoryImpl::new(mongo_client.clone()));
    let user_repo = Arc::new(UserRepositoryImpl::new(mongo_client.clone()));
    let role_repo = Arc::new(UserTenantRoleRepositoryImpl::new(mongo_client.clone()));
    let approval_repo = Arc::new(ApprovalRepositoryImpl::new(mongo_client.clone()));
    let variable_repo = Arc::new(VariableRepositoryImpl::new(mongo_client.clone()));
    let workflow_def_repo = Arc::new(WorkflowDefinitionRepositoryImpl::new(mongo_client.clone()));
    let workflow_inst_repo = Arc::new(WorkflowInstanceRepositoryImpl::new(mongo_client.clone()));

    let task_service = TaskService::new(task_repo);
    let task_instance_service = TaskInstanceService::new(task_instance_repo);
    let tenant_service = TenantService::new(tenant_repo);
    let user_service = UserService::new(user_repo, role_repo.clone());
    let approval_service = ApprovalService::new(approval_repo, role_repo);
    let variable_service = VariableService::new(variable_repo, encrypt_key);
    let workflow_def_service = WorkflowDefinitionService::new(workflow_def_repo);
    let workflow_inst_service = WorkflowInstanceService::new(workflow_inst_repo);

    let auth_handler = Arc::new(AuthHandler::new(user_service.clone()));
    let tenant_handler = Arc::new(TenantHandler::new(tenant_service));
    let user_handler = Arc::new(UserHandler::new(user_service));
    let approval_handler = Arc::new(ApprovalHandler::new(approval_service, dispatcher.clone()));
    let variable_handler = Arc::new(VariableHandler::new(variable_service));
    let task_handler = Arc::new(TaskHandler::new(task_service));
    let task_instance_handler = Arc::new(TaskInstanceHandler::new(task_instance_service, dispatcher.clone()));
    let workflow_handler = Arc::new(WorkflowHandler::new(workflow_def_service.clone()));
    let workflow_instance_handler = Arc::new(WorkflowInstanceHandler::new(
        workflow_def_service,
        workflow_inst_service,
        dispatcher,
    ));

    let app = create_router(
        auth_handler,
        tenant_handler,
        user_handler,
        variable_handler,
        approval_handler,
        task_handler,
        task_instance_handler,
        workflow_handler,
        workflow_instance_handler,
    );

    let port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to {}", addr));

    println!("API server ready at {}", addr);
    axum::serve(listener, app).await.expect("server error");
}
