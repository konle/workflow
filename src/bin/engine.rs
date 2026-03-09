use std::sync::Arc;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use domain::plugin::manager::PluginManager;
use domain::shared::job::{ExecuteTaskJob, ExecuteWorkflowJob, TaskDispatcher};
use domain::task::service::TaskInstanceService;
use domain::workflow::service::WorkflowService;
use infrastructure::queue::consumer;
use async_trait::async_trait;
struct ApalisDispatcher {
    task_storage: RedisStorage<ExecuteTaskJob>,
    workflow_storage: RedisStorage<ExecuteWorkflowJob>,
}

#[async_trait]
impl TaskDispatcher for ApalisDispatcher {
    async fn dispatch_task(&self, job: ExecuteTaskJob) -> anyhow::Result<()> {
        let mut storage = self.task_storage.clone();
        storage.push(job).await.map_err(|e| anyhow::anyhow!("Failed to push task: {}", e))?;
        Ok(())
    }

    async fn dispatch_workflow(&self, job: ExecuteWorkflowJob) -> anyhow::Result<()> {
        let mut storage = self.workflow_storage.clone();
        storage.push(job).await.map_err(|e| anyhow::anyhow!("Failed to push workflow: {}", e))?;
        Ok(())
    }
}

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
    workflow_svc: Arc<WorkflowService>,
    task_storage: RedisStorage<ExecuteTaskJob>,
    workflow_storage: RedisStorage<ExecuteWorkflowJob>,
) -> Arc<PluginManager> {
    let dispatcher = Arc::new(ApalisDispatcher {
        task_storage,
        workflow_storage,
    });
    let mut manager = PluginManager::new(workflow_svc, dispatcher);
    manager.register(Box::new(domain::plugin::plugins::http::HttpPlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::parallel::ParallelPlugin::new()));
    manager.register(Box::new(domain::plugin::plugins::ifcondition::IfConditionPlugin::new()));
    Arc::new(manager)
}

fn create_task_manager(task_instance_svc: Arc<TaskInstanceService>) -> Arc<TaskManager> {
    let mut manager = TaskManager::new(task_instance_svc);
    manager.register(Box::new(HttpTaskExecutor::new()));
    Arc::new(manager)
}

#[tokio::main]
async fn main() {
    println!("Workflow engine starting...");

    let mongo_url = std::env::var("MONGO_URL").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let mongo_client = mongodb::Client::with_uri_str(&mongo_url).await.expect("failed to connect to MongoDB");
    let workflow_repo = Arc::new(
        infrastructure::mongodb::workflow::workflow_repository_impl::WorkflowRepositoryImpl::new(mongo_client.clone())
    );
    let workflow_svc = Arc::new(WorkflowService::new(workflow_repo));
    
    let task_repo = Arc::new(infrastructure::mongodb::task::task_repository_impl::TaskInstanceRepositoryImpl::new(mongo_client.clone()));
    let task_svc = Arc::new(TaskInstanceService::new(task_repo));
    let task_manager = create_task_manager(task_svc);
    
    let wf_storage = consumer::create_workflow_storage(&redis_url).await;
    let task_storage = consumer::create_task_storage(&redis_url).await;

    let plugin_manager = create_plugin_manager(workflow_svc, task_storage.clone(), wf_storage.clone());

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
