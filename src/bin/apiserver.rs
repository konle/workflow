#[tokio::main]
async fn main() {
    println!("API server starting...");

    // TODO: 初始化 MongoDB 连接
    // TODO: 初始化 Redis 连接 + JobProducer
    //
    // 示例：
    //   let redis_client = redis::Client::open("redis://127.0.0.1:6379").unwrap();
    //   let wf_storage = RedisStorage::<ExecuteWorkflowJob>::new(redis_client.clone());
    //   let task_storage = RedisStorage::<ExecuteTaskJob>::new(redis_client.clone());
    //   let job_producer = JobProducer::new(wf_storage, task_storage);
    //
    //   let repo = Arc::new(TaskRepositoryImpl::new(mongo_client));
    //   let service = TaskService::new(repo);
    //   let task_handler = Arc::new(TaskHandler::new(service));
    //
    //   let app = api::router::create_router(task_handler);
    //   let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    //   axum::serve(listener, app).await.unwrap();

    println!("API server ready at 0.0.0.0:3000");
}
