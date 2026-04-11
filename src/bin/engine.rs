use std::sync::Arc;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use clap::Parser;
use tracing::{info, warn, error};
use domain::plugin::manager::PluginManager;
use domain::shared::job::{ExecuteTaskJob, ExecuteWorkflowJob};
use domain::sweeper::{Sweeper, SweeperConfig};
use domain::task::service::TaskInstanceService;
use domain::approval::service::ApprovalService;
use domain::variable::service::VariableService;
use domain::workflow::service::{WorkflowDefinitionService, WorkflowInstanceService};
use infrastructure::queue::consumer;
use infrastructure::queue::dispatcher::ApalisDispatcher;
use workflow::config::AppConfig;

async fn handle_workflow_job(job: ExecuteWorkflowJob, manager: Data<Arc<PluginManager>>) -> Result<(), std::io::Error> {
    info!(
        workflow_instance_id = %job.workflow_instance_id,
        event = ?job.event,
        "processing workflow job"
    );

    let worker_id = "workflow-worker-1";
    if let Err(e) = manager.process_workflow_job(job.clone(), worker_id).await {
        error!(
            workflow_instance_id = %job.workflow_instance_id,
            error = %e,
            "workflow job failed"
        );
        return Err(std::io::Error::other(e));
    }

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
    let task_svc = task_manager.task_instance_svc();
    info!(task_instance_id = %job.task_instance_id, "processing task job");

    // CAS claim: Pending → Running (only one worker can succeed)
    let task_instance_entity = match task_svc.submit_instance(&job.task_instance_id).await {
        Ok(inst) => inst,
        Err(e) => {
            warn!(task_instance_id = %job.task_instance_id, error = %e,
                "task instance not claimable (already running or terminal), skipping");
            return Ok(());
        }
    };

    let exec_result = match task_manager.execute_task(&task_instance_entity).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                task_instance_id = %job.task_instance_id,
                task_type = ?task_instance_entity.task_type,
                error = %e,
                "task execution failed"
            );

            // CAS: Running → Failed
            let _ = task_svc.fail_instance(&job.task_instance_id).await;
            if let Ok(mut ti) = task_svc.get_task_instance_entity(job.task_instance_id.clone()).await {
                ti.error_message = Some(e.to_string());
                let _ = task_svc.update_task_instance_entity(ti).await;
            }

            if let Some(caller) = job.caller_context {
                let _ = manager
                    .dispatcher()
                    .dispatch_workflow(ExecuteWorkflowJob {
                        workflow_instance_id: caller.workflow_instance_id.clone(),
                        tenant_id: job.tenant_id,
                        event: domain::shared::job::WorkflowEvent::NodeCallback {
                            node_id: caller.node_id,
                            child_task_id: job.task_instance_id.clone(),
                            status: domain::workflow::entity::workflow_definition::NodeExecutionStatus::Failed,
                            output: None,
                            error_message: Some(e.to_string()),
                            input: None,
                        },
                    })
                    .await;
            }

            return Err(std::io::Error::other(e));
        }
    };

    // CAS finalize: Running → Completed or Running → Failed
    let final_status = match exec_result.status {
        domain::workflow::entity::workflow_definition::NodeExecutionStatus::Success => {
            let _ = task_svc.complete_instance(&job.task_instance_id).await;
            domain::shared::workflow::TaskInstanceStatus::Completed
        }
        _ => {
            let _ = task_svc.fail_instance(&job.task_instance_id).await;
            domain::shared::workflow::TaskInstanceStatus::Failed
        }
    };

    // Write output/error fields (non-status update)
    if let Ok(mut entity) = task_svc.get_task_instance_entity(job.task_instance_id.clone()).await {
        entity.output = exec_result.output.clone();
        entity.input = exec_result.input.clone();
        entity.error_message = exec_result.error_message.clone();
        let _ = task_svc.update_task_instance_entity(entity).await;
    }

    info!(
        task_instance_id = %job.task_instance_id,
        status = ?final_status,
        "task completed"
    );

    if let Some(caller) = job.caller_context {
        manager
            .dispatcher()
            .dispatch_workflow(ExecuteWorkflowJob {
                workflow_instance_id: caller.workflow_instance_id.clone(),
                tenant_id: job.tenant_id,
                event: domain::shared::job::WorkflowEvent::NodeCallback {
                    node_id: caller.node_id,
                    child_task_id: job.task_instance_id.clone(),
                    status: exec_result.status,
                    output: exec_result.output,
                    error_message: exec_result.error_message,
                    input: exec_result.input,
                },
            })
            .await
            .map_err(|e| {
                error!(
                    task_instance_id = %job.task_instance_id,
                    workflow_instance_id = %caller.workflow_instance_id,
                    error = %e,
                    "failed to dispatch workflow callback"
                );
                std::io::Error::other(e)
            })?;
    }

    Ok(())
}

fn create_plugin_manager(
    workflow_definition_svc: WorkflowDefinitionService,
    workflow_instance_svc: Arc<WorkflowInstanceService>,
    task_instance_svc: Arc<TaskInstanceService>,
    variable_svc: VariableService,
    approval_svc: ApprovalService,
    task_storage: RedisStorage<ExecuteTaskJob>,
    workflow_storage: RedisStorage<ExecuteWorkflowJob>,
) -> Arc<PluginManager> {
    let dispatcher = Arc::new(ApalisDispatcher::new(task_storage, workflow_storage));
    let mut manager = PluginManager::new(workflow_instance_svc.clone(), dispatcher)
        .with_task_instance_service(task_instance_svc)
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

    workflow::init_tracing(&config.log);

    info!(config = %cli.config, "engine starting");

    let mongo_client = mongodb::Client::with_uri_str(&config.database.mongo_url)
        .await
        .unwrap_or_else(|e| {
            error!(url = %config.database.mongo_url, error = %e, "failed to connect to MongoDB");
            std::process::exit(1);
        });
    info!("connected to MongoDB");

    let workflow_def_repo = Arc::new(
        infrastructure::mongodb::workflow::workflow_repository_impl::WorkflowDefinitionRepositoryImpl::new(mongo_client.clone())
    );
    let workflow_definition_svc = WorkflowDefinitionService::new(workflow_def_repo);

    let task_repo = Arc::new(infrastructure::mongodb::task::task_repository_impl::TaskInstanceRepositoryImpl::new(mongo_client.clone()));
    let task_svc = Arc::new(TaskInstanceService::new(task_repo));
    let task_manager = create_task_manager(task_svc.clone());

    let workflow_repo = Arc::new(
        infrastructure::mongodb::workflow::workflow_repository_impl::WorkflowInstanceRepositoryImpl::new(mongo_client.clone())
    );
    let workflow_instance_svc = Arc::new(WorkflowInstanceService::new(workflow_repo, task_svc.clone()));

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

    let wf_storage = consumer::create_workflow_storage(&config.database.redis_url).await;
    let task_storage = consumer::create_task_storage(&config.database.redis_url).await;
    info!("connected to Redis");

    let plugin_manager = create_plugin_manager(
        workflow_definition_svc,
        workflow_instance_svc.clone(),
        task_svc.clone(),
        variable_svc,
        approval_svc.clone(),
        task_storage.clone(),
        wf_storage.clone(),
    );

    let wf_worker = WorkerBuilder::new("workflow-worker")
        .data(plugin_manager.clone())
        .backend(wf_storage)
        .build_fn(handle_workflow_job);

    let task_worker = WorkerBuilder::new("task-worker")
        .data((plugin_manager.clone(), task_manager))
        .backend(task_storage)
        .build_fn(handle_task_job);

    if config.sweeper.enabled {
        let sweeper = Arc::new(
            Sweeper::new(
                workflow_instance_svc.clone(),
                task_svc.clone(),
                plugin_manager.dispatcher(),
                SweeperConfig {
                    interval_secs: config.sweeper.interval_secs,
                    max_recover_per_cycle: config.sweeper.max_recover_per_cycle,
                },
            )
            .with_approval_service(approval_svc.clone()),
        );
        let interval_secs = config.sweeper.interval_secs;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(interval_secs),
            );
            interval.tick().await; // skip first immediate tick
            loop {
                interval.tick().await;
                sweeper.run_cycle().await;
            }
        });
        info!(
            interval_secs = config.sweeper.interval_secs,
            max_recover = config.sweeper.max_recover_per_cycle,
            "sweeper started"
        );
    }

    info!("engine ready, waiting for jobs");

    Monitor::new()
        .register(wf_worker)
        .register(task_worker)
        .run()
        .await
        .unwrap_or_else(|e| {
            error!(error = %e, "monitor failed");
        });
}
