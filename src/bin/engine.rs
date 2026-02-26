use std::sync::Arc;
use apalis::prelude::*;
use domain::plugin::manager::PluginManager;
use domain::plugin::plugins::http::HttpPlugin;
use domain::shared::job::{ExecuteTaskJob, ExecuteWorkflowJob};
use domain::workflow::service::WorkflowService;
use infrastructure::queue::consumer;

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

async fn handle_task_job(job: ExecuteTaskJob, _manager: Data<Arc<PluginManager>>) -> Result<(), std::io::Error> {
    println!("Executing task: {}", job.task_instance_id);

    // TODO: 从 MongoDB 加载 TaskInstanceEntity 
    // 利用 manager.execute_node_instance 执行单个节点

    Ok(())
}

fn create_plugin_manager(workflow_svc: Arc<WorkflowService>) -> Arc<PluginManager> {
    let mut manager = PluginManager::new(workflow_svc);
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
    let plugin_manager = create_plugin_manager(workflow_svc);

    let wf_storage = consumer::create_workflow_storage(&redis_url).await;
    let task_storage = consumer::create_task_storage(&redis_url).await;

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
