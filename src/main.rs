/*
 * @Description: 
 * @Author: zhangfu 18072150332@163.com
 * @Date: 2026-01-09 22:06:06
 * @LastEditors: zhangfu 18072150332@163.com
 * @LastEditTime: 2026-01-14 22:29:19
 */
use axum::{routing::{ get, post }, Router, serve, body::Bytes, extract::{ Path, Query, Json, Multipart, Form }, http::StatusCode, response::IntoResponse };
use tokio::net::TcpListener;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

// 返回 &'static str（静态字符串，Axum 自动转为 200 OK 响应）
#[allow(unused)]
async fn hello_world() -> &'static str {
    "Hello, Axum! 🚀"
}

async fn hello_name(Path(name): Path<String>) -> String {
    format!("Hello, {}! 👋", name)
}

// Query是一个Map，所有在rust中我们使用HashMap来接收
async fn greet(params: Query<std::collections::HashMap<String, String>>) -> String {
    // 处理两个参数都有，通过元组匹配
    if let (Some(name), Some(sex)) = (params.get("name"), params.get("sex")) {
        format!("Greetings, {} the {}! 🌟", name, sex)
    }
    // 处理只有name
    else if let Some(name) = params.get("name") {
        format!("Greetings, {}! 🌟", name)
    } 
    // 处理只有sex
    else if let Some(sex) = params.get("sex") {
        format!("Greetings, {}! 🌟", sex)
    }
    // 处理没有参数
    else {
        "Greetings, stranger! 🌟".to_string()
    }
}

#[derive(Deserialize, Serialize)]
struct Payload {
    msg: String,
}

// JSON 数据处理
async fn echo_json(Json(payload): Json<Payload>) -> impl IntoResponse {
    (StatusCode::OK, Json(Payload {
        msg: format!("Echo: {}", payload.msg),
    }))
}

// form-data 数据处理
async fn form_data(mut multipart: Multipart) -> impl IntoResponse {
    println!("Processing form data:");
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("unknown").to_string();
        let data = field.text().await.unwrap_or_default();
        println!("Field Name: {}, Data: {}", name, data);
    }
    (StatusCode::OK, "Form data processed successfully.")
}

#[derive(Debug, Deserialize)]
struct User {
    name: String,
    say: String,
}
// form-urlencoded 数据处理
async fn form_urlencoded(Form(user): Form<User>) -> impl IntoResponse {
    println!("Received user: {:?}", user);
    (StatusCode::OK, format!("Received name: {}, say: {}", user.name, user.say))
}
// 二进制数据处理
async fn echo_binary(body: Bytes) -> impl IntoResponse {
    (StatusCode::OK, format!("Received {} bytes", body.len()))
}

// 自定义 fallback 处理函数
async fn fallback() -> impl IntoResponse {
    println!("没有匹配到路由");
    (StatusCode::NOT_FOUND, "页面未找到")
}

#[tokio::main]
    // 给 main 加返回值：Result<(), 错误类型>
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建路由器（Router）：管理路由规则
    let app = Router::new()
    .route("/echo", post(echo_json))
    .route("/greet", get(greet))
    .route("/hello/{name}", get(hello_name))
    .route("/form-data", post(form_data))
    .route("/form-urlencoded", post(form_urlencoded))
    .route("/binary", post(echo_binary))
    // 绑定路由：GET 方法 + 路径 "/" + 处理函数 hello_world
    // .route("/", get(hello_world))
    .nest_service("/assets", ServeDir::new("static/assets"))

    .fallback(fallback);


    // 绑定端口（返回 Result，需用 ? 处理错误）
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("服务器启动成功！访问：http://127.0.0.1:3000");
    // 启动服务器
    serve(listener, app.into_make_service()).await?;

    Ok(())
}