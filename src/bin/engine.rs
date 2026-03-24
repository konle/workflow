use std::sync::Arc;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use clap::Parser;
use domain::plugin::manager::PluginManager;
use domain::shared::job::{ExecuteTaskJob, ExecuteWorkflowJob};
use domain::task::service::TaskInstanceService;
use domain::approval::service::ApprovalService;
use domain::variable::service::VariableService;
use domain::workflow::service::{WorkflowDefinitionService, WorkflowInstanceService};
use infrastructure::queue::consumer;
use infrastructure::queue::dispatcher::ApalisDispatcher;
use workflow::config::AppConfig;

async fn handle_workflow_job(job: ExecuteWorkflowJob, manager: Data<Arc<PluginManager>>) -> Result<(), std::io::Error> {
    println!("Executing workflow job: {} event: {:?}", job.workflow_instance_id, job.event);
    
    let worker_id = "workflow-worker-1"; // In a real cluster, this should be a unique ID like uuid
    manager
        .process_workflow_job(job, worker_id)
        .await
        .map_err(|e| std::io::Error::other(e))?;

    Ok(())
}

use domain::task::manager::TaskManager;
use domain::task::executors::http::HttpTaskExecutor;

async fn handle_task_job(
    job: ExecuteTaskJob, 
    ctx: Data<(Arc<PluginManager>, Arc<TaskManager>)>
) -> Result<(), std::io::Error> {
    let manager = &ctx.0;
    let task_manager = &ctx.1;
    println!("Executing task: {}", job.task_instance_id);

    // 1. 获取任务的 Config
    let mut task_instance_entity = task_manager
        .task_instance_svc()
        .get_task_instance_entity(job.task_instance_id.clone())
        .await
        .map_err(|e| std::io::Error::other(e))?;

    // 2. 分发并执行任务
    let exec_result = task_manager
        .execute_task(&task_instance_entity)
        .await
        .map_err(|e| std::io::Error::other(e))?;

    // 3. 更新 TaskInstance 本身状态
    task_instance_entity.task_status = match exec_result.status {
        domain::workflow::entity::NodeExecutionStatus::Success => domain::shared::workflow::TaskInstanceStatus::Completed,
        domain::workflow::entity::NodeExecutionStatus::Failed => domain::shared::workflow::TaskInstanceStatus::Failed,
        _ => domain::shared::workflow::TaskInstanceStatus::Pending,
    };
    task_instance_entity.output = exec_result.output.clone().map(|o| o.data);
    task_instance_entity.input = exec_result.input.clone();
    task_instance_entity.error_message = exec_result.error_message.clone();
    
    // 保存 task_instance_entity 
    task_manager.task_instance_svc().update_task_instance_entity(task_instance_entity.clone()).await.map_err(|e| std::io::Error::other(e))?;

    // 4. 回调处理
    if let Some(ctx) = job.caller_context {
        manager
            .dispatcher()
            .dispatch_workflow(ExecuteWorkflowJob {
                workflow_instance_id: ctx.workflow_instance_id,
                tenant_id: job.tenant_id,
                event: domain::shared::job::WorkflowEvent::NodeCallback {
                    node_id: ctx.node_id,
                    child_task_id: job.task_instance_id,
                    status: exec_result.status,
                    output: exec_result.output.map(|o| o.data),
                    error_message: exec_result.error_message,
                    input: exec_result.input,
                },
            })
            .await
            .map_err(|e| std::io::Error::other(e))?;
    }

    Ok(())
}

fn create_plugin_manager(
    workflow_definition_svc: WorkflowDefinitionService,
    workflow_instance_svc: Arc<WorkflowInstanceService>,
    variable_svc: VariableService,
    approval_svc: ApprovalService,
    task_storage: RedisStorage<ExecuteTaskJob>,
    workflow_storage: RedisStorage<ExecuteWorkflowJob>,
) -> Arc<PluginManager> {
    let dispatcher = Arc::new(ApalisDispatcher::new(task_storage, workflow_storage));
    let mut manager = PluginManager::new(workflow_instance_svc.clone(), dispatcher)
        .with_variable_service(variable_svc);
    manager.register(Box::new(domain::plugin::plugins::http::HttpPlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::parallel::ParallelPlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::ifcondition::IfConditionPlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::contextrewrite::ContextRewritePlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::forkjoin::ForkJoinPlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::approval::ApprovalPlugin::new(approval_svc)));
    manager.register(Box::new(domain::plugin::plugins::subworkflow::SubWorkflowPlugin::new(
        workflow_definition_svc,
        (*workflow_instance_svc).clone(),
    )));
    Arc::new(manager)
}

fn create_task_manager(task_instance_svc: Arc<TaskInstanceService>) -> Arc<TaskManager> {
    let mut manager = TaskManager::new(task_instance_svc);
    manager.register(Box::new(HttpTaskExecutor::new()));
    Arc::new(manager)
}

#[derive(Parser)]
#[command(name = "engine", about = "Workflow Engine")]
struct Cli {
    #[arg(long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = AppConfig::load(&cli.config).expect("failed to load config");

    println!("Workflow engine starting...");

    let mongo_client = mongodb::Client::with_uri_str(&config.database.mongo_url).await.expect("failed to connect to MongoDB");

    let workflow_def_repo = Arc::new(
        infrastructure::mongodb::workflow::workflow_repository_impl::WorkflowDefinitionRepositoryImpl::new(mongo_client.clone())
    );
    let workflow_definition_svc = WorkflowDefinitionService::new(workflow_def_repo);

    let workflow_repo = Arc::new(
        infrastructure::mongodb::workflow::workflow_repository_impl::WorkflowInstanceRepositoryImpl::new(mongo_client.clone())
    );
    let workflow_instance_svc = Arc::new(WorkflowInstanceService::new(workflow_repo));

    let variable_repo = Arc::new(
        infrastructure::mongodb::variable::variable_repository_impl::VariableRepositoryImpl::new(mongo_client.clone())
    );
    let variable_svc = VariableService::new(variable_repo, config.security.variable_encrypt_key.clone());

    let role_repo = Arc::new(
        infrastructure::mongodb::user::user_repository_impl::UserTenantRoleRepositoryImpl::new(mongo_client.clone())
    );
    let approval_repo = Arc::new(
        infrastructure::mongodb::approval::approval_repository_impl::ApprovalRepositoryImpl::new(mongo_client.clone())
    );
    let approval_svc = ApprovalService::new(approval_repo, role_repo);

    let task_repo = Arc::new(infrastructure::mongodb::task::task_repository_impl::TaskInstanceRepositoryImpl::new(mongo_client.clone()));
    let task_svc = Arc::new(TaskInstanceService::new(task_repo));
    let task_manager = create_task_manager(task_svc);

    let wf_storage = consumer::create_workflow_storage(&config.database.redis_url).await;
    let task_storage = consumer::create_task_storage(&config.database.redis_url).await;

    let plugin_manager = create_plugin_manager(workflow_definition_svc, workflow_instance_svc, variable_svc, approval_svc, task_storage.clone(), wf_storage.clone());

    let wf_worker = WorkerBuilder::new("workflow-worker")
        .data(plugin_manager.clone())
        .backend(wf_storage)
        .build_fn(handle_workflow_job);

    let task_worker = WorkerBuilder::new("task-worker")
        .data((plugin_manager.clone(), task_manager))
        .backend(task_storage)
        .build_fn(handle_task_job);

    println!("Workflow engine ready. Waiting for jobs...");

    Monitor::new()
        .register(wf_worker)
        .register(task_worker)
        .run()
        .await
        .expect("Monitor failed");
}
