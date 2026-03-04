use std::sync::Arc;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use domain::plugin::manager::PluginManager;
use domain::plugin::plugins::http::HttpPlugin;
use domain::shared::job::{ExecuteTaskJob, ExecuteWorkflowJob, TaskDispatcher};
use domain::workflow::service::WorkflowService;
use domain::workflow::entity::NodeExecutionStatus;
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
    println!("Executing workflow: {}", job.workflow_instance_id);

    let mut workflow_instance = manager
        .workflow_svc()
        .get_workflow_instance(job.workflow_instance_id)
        .await
        .map_err(|e| std::io::Error::other(e))?;

    manager
        .execute_workflow(&mut workflow_instance)
        .await
        .map_err(|e| std::io::Error::other(e))?;

    Ok(())
}

async fn handle_task_job(job: ExecuteTaskJob, manager: Data<Arc<PluginManager>>) -> Result<(), std::io::Error> {
    println!("Executing task: {}", job.task_instance_id);

    // 1. 获取任务的 Config
    // TODO: 从 MongoDB 加载 TaskInstanceEntity
    // 这里我们先模拟，如果这是一个来自于工作流的任务，我们需要利用 HttpPlugin 真实的执行逻辑
    // 为了完整示例，我们在后续版本中应该从 DB 获取 task template。
    
    // 我们假设它是被工作流触发的
    if let Some(ctx) = job.caller_context {
        let mut instance = manager
            .workflow_svc()
            .get_workflow_instance(ctx.workflow_instance_id.clone())
            .await
            .map_err(|e| std::io::Error::other(e))?;

        if let Some(node_index) = instance.nodes.iter().position(|n| n.node_id == ctx.node_id) {
            let node_config = instance.nodes[node_index].config.clone();

            // 如果是 HTTP 任务
            if let domain::task::entity::TaskTemplate::Http(http_config) = node_config {
                let http_plugin = HttpPlugin::new();
                
                // 执行真正的请求
                match http_plugin.do_request(&http_config).await {
                    Ok((status, output, err_msg)) => {
                        instance.nodes[node_index].status = status;
                        instance.nodes[node_index].output = output;
                        instance.nodes[node_index].error_message = err_msg;
                    }
                    Err(e) => {
                        instance.nodes[node_index].status = NodeExecutionStatus::Failed;
                        instance.nodes[node_index].error_message = Some(e.to_string());
                    }
                }

                // 保存更新后的工作流实例状态
                manager
                    .workflow_svc()
                    .save_workflow_instance(&instance)
                    .await
                    .map_err(|e| std::io::Error::other(e))?;

                // 唤醒工作流，让它继续推进
                manager
                    .dispatcher()
                    .dispatch_workflow(ExecuteWorkflowJob {
                        workflow_instance_id: ctx.workflow_instance_id,
                        tenant_id: job.tenant_id,
                    })
                    .await
                    .map_err(|e| std::io::Error::other(e))?;
            }
        }
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
    manager.register(Box::new(HttpPlugin::new()));
    Arc::new(manager)
}

#[tokio::main]
async fn main() {
    println!("Workflow engine starting...");

    let mongo_url = std::env::var("MONGO_URL").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let mongo_client = mongodb::Client::with_uri_str(&mongo_url).await.expect("failed to connect to MongoDB");
    let workflow_repo = Arc::new(
        infrastructure::mongodb::workflow::workflow_repository_impl::WorkflowRepositoryImpl::new(mongo_client)
    );
    let workflow_svc = Arc::new(WorkflowService::new(workflow_repo));
    
    let wf_storage = consumer::create_workflow_storage(&redis_url).await;
    let task_storage = consumer::create_task_storage(&redis_url).await;

    let plugin_manager = create_plugin_manager(workflow_svc, task_storage.clone(), wf_storage.clone());

    let wf_worker = WorkerBuilder::new("workflow-worker")
        .data(plugin_manager.clone())
        .backend(wf_storage)
        .build_fn(handle_workflow_job);

    let task_worker = WorkerBuilder::new("task-worker")
        .data(plugin_manager.clone())
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
