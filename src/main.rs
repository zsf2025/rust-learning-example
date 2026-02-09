use axum:: {
    http:: { Request, StatusCode },
    middleware::Next,
    response::Response,
};

use std::time::Instant;

use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Semaphore;

use axum::{routing::get, Router, middleware};
use std::net::SocketAddr;

async fn logger_and_timer_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. 前置逻辑：记录开始时间
    let start = Instant::now();
    let mthod = req.method().clone();
    let path = req.uri().path().to_owned();

    // 2. 执行后续中间件及Handler
    let mut response = next.run(req).await;

    // 3. 后置逻辑：计算耗时
    let duration = start.elapsed();
    let duration_ms = format!("{:?}", duration);

    // 任务2：在响应头添加X-Response-Time

    response.headers_mut().insert(
        "X-Response-Time",
        axum::http::HeaderValue::from_str(&duration_ms).unwrap(),
    );

    // 任务1: 打印日志
    println!("[{}] {} {}| Status: {} | Duration:{:?}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        mthod,
        path,
        response.status(),
        duration
    );
    Ok(response)
}

#[derive(Clone)]
struct AppState {
    semaphore: Arc<Semaphore>,
}

async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode>{
    // 尝试获取一个许可
   // try_acquire 会立即返回，如果信号量已满，则返回错误
   match state.semaphore.try_acquire() {
    Ok(_permit) => {
        // 获取成功，执行后续逻辑
        // 注意：_permit 会在此函数结束时自动Drop，从而释放信号量
        Ok(next.run(req).await)
    }
    Err(_) => {
        // 获取失败，说明并发量已达上限
        eprintln!("Too many requests!");
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
   }
}



#[tokio::main]
async fn main() {
    // 初始化状态：限制最大并发数为 10
    let state = AppState {
        semaphore: Arc::new(Semaphore::new(3)),
    };

    let app = Router::new()
    .route("/", get(|| async { "Hello, World!" }))
    .route("/slow", get(|| async {
        // 模拟一个慢请求
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        "This is a slow response."
    }))
    // 应用限流中间件（带状态）
    .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
    .layer(middleware::from_fn(logger_and_timer_middleware))
    .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}