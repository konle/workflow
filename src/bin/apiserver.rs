use std::sync::Arc;
use tokio::net::TcpListener;
use clap::Parser;

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
use domain::user::entity::TenantRole;
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

use workflow::config::AppConfig;

#[derive(Parser)]
#[command(name = "apiserver", about = "Workflow API Server")]
struct Cli {
    #[arg(long, default_value = "config.toml")]
    config: String,

    #[arg(long, help = "Initialize default tenant and super admin")]
    init: bool,
}

async fn bootstrap(
    config: &AppConfig,
    user_service: &UserService,
    tenant_service: &TenantService,
) {
    let init = &config.init;

    // 1. Create super admin user (skip if exists)
    let admin = match user_service.get_user_by_username(&init.admin_username).await {
        Ok(existing) => {
            println!("[init] super admin '{}' already exists, skipping", init.admin_username);
            existing
        }
        Err(_) => {
            let password_hash = bcrypt::hash(&init.admin_password, bcrypt::DEFAULT_COST)
                .expect("failed to hash admin password");
            let user = user_service
                .create_user(
                    init.admin_username.clone(),
                    init.admin_email.clone(),
                    password_hash,
                    true,
                )
                .await
                .expect("failed to create super admin user");
            println!("[init] created super admin '{}'", init.admin_username);
            user
        }
    };

    // 2. Create default tenant (skip if exists)
    let tenant = match tenant_service.get_tenant(&init.default_tenant_name).await {
        Ok(existing) => {
            println!("[init] tenant '{}' already exists, skipping", init.default_tenant_name);
            existing
        }
        Err(_) => {
            let t = tenant_service
                .create_tenant(init.default_tenant_name.clone(), init.default_tenant_description.clone())
                .await
                .expect("failed to create default tenant");
            println!("[init] created tenant '{}' (id: {})", init.default_tenant_name, t.tenant_id);
            t
        }
    };

    // 3. Assign TenantAdmin role on the default tenant (skip if exists)
    match user_service.get_role(&admin.user_id, &tenant.tenant_id).await {
        Ok(_) => {
            println!("[init] admin already has role on tenant '{}', skipping", tenant.tenant_id);
        }
        Err(_) => {
            user_service
                .assign_role(&admin.user_id, &tenant.tenant_id, &TenantRole::TenantAdmin)
                .await
                .expect("failed to assign admin role");
            println!("[init] assigned TenantAdmin role to '{}' on tenant '{}'", init.admin_username, tenant.tenant_id);
        }
    }

    println!("[init] bootstrap complete");
    println!("[init] ────────────────────────────────────");
    println!("[init]   username : {}", init.admin_username);
    println!("[init]   password : {}", init.admin_password);
    println!("[init]   tenant   : {} (id: {})", init.default_tenant_name, tenant.tenant_id);
    println!("[init] ────────────────────────────────────");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = AppConfig::load(&cli.config).expect("failed to load config");

    println!("API server starting...");

    let mongo_client = mongodb::Client::with_uri_str(&config.database.mongo_url)
        .await
        .expect("failed to connect to MongoDB");

    let task_storage = consumer::create_task_storage(&config.database.redis_url).await;
    let workflow_storage = consumer::create_workflow_storage(&config.database.redis_url).await;
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
    let variable_service = VariableService::new(variable_repo, config.security.variable_encrypt_key.clone());
    let workflow_def_service = WorkflowDefinitionService::new(workflow_def_repo);
    let workflow_inst_service = WorkflowInstanceService::new(workflow_inst_repo);

    if cli.init {
        bootstrap(&config, &user_service, &tenant_service).await;
    }

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

    let addr = format!("0.0.0.0:{}", config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to {}", addr));

    println!("API server ready at {}", addr);
    axum::serve(listener, app).await.expect("server error");
}
