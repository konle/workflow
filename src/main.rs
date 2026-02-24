use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // TODO: 通过 DI 创建 TaskHandler，注入 infrastructure 层的 Repository 实现
    //
    // 示例：
    //   let repo = Arc::new(TaskRepositoryImpl::new(mongo_client));
    //   let service = TaskService::new(repo);
    //   let task_handler = Arc::new(TaskHandler::new(service));
    //
    //   let app = api::router::create_router(task_handler);
    //   let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    //   axum::serve(listener, app).await.unwrap();

    println!("Server ready to start at 0.0.0.0:3000");
}
