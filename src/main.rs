mod handlers;
mod models;

use axum:: { routing::{ get, post }, Router, serve };
use tera::{Tera};
use crate::{
    models::{
        Article
    }
};
use std:: {
    sync::{Arc, RwLock}
};

#[tokio::main]
async fn main() {
    // 初始化Tera模板引擎
    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("模板解析错误: {}", e);
            std::process::exit(1);
        }
    };

    // 初始化模拟数据
    let initial_posts = vec![
        Article {
            id: 1,
            title: "第一篇文章".to_string(),
            content: "这是我的第一篇文章内容。".to_string()
        }
    ];

    // 构建共享状态
    let state = Arc::new(handlers::AppState {
        tera,
        posts: RwLock::new(initial_posts),
    });

   // 定义路由
   let app = Router::new()
        .route("/", get(handlers::home_handler))
        .route("/post", post(handlers::create_post_handler))
        .route("/post/{id}", get(handlers::post_handler))
        .with_state(state);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("服务器运行在 http://{}", listener.local_addr().unwrap());
    serve(listener, app).await.unwrap();
}