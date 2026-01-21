mod error;
mod handlers;
mod models;

use axum:: { routing::{ get }, Router, serve };
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::{sqlite::SqlitePoolOptions };
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{

    // 初始化环境变量
    dotenvy::dotenv().ok();

    // 初始化日志
    // 创建一个tracing订阅者注册表
    tracing_subscriber::registry()
        // 添加一个格式化层，方便人类查看和调试
        .with(tracing_subscriber::fmt::layer())
        // 从默认环境变量中创建一个环境过滤器。环境过滤器用于根据环境变量来决定哪些日志记录应该被处理和输出。
        // 这里的指令是将 todo_api 模块的日志级别设置为 debug。意味着 todo_api 模块产生的所有 debug 及以上级别的日志（如 info、warn、error）都会被处理和输出。
        // 如果不add_directive, 默认通常处理和输出error级别的日志
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("todo_api=debug".parse()?))
        // 应用前面配置进行初始化收集日志
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("环境变量必须设置DATABASE");

    // 数据库连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // 
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                done BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
        "#
    )
    .execute(&pool)
    .await?;

    tracing::info!("数据迁移成功!");

    // 构建应用状态
    let state = handlers::AppState { pool };

    // 构建路由
    let app = Router::new()
        .route("/todos", get(handlers::list_todos).post(handlers::create_todo))
        .route("/todos/{id}", get(handlers::get_todo).put(handlers::update_todo).delete(handlers::delete_todo))
        .with_state(state); // 注入状态
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    tracing::info!("启动服务器:{}", listener.local_addr()?);
    println!("启动服务: http://{}", listener.local_addr()?);

    serve(listener, app).await?;

    Ok(())
}