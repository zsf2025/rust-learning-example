use axum::{
    response::{IntoResponse, Response},
    Json,
    extract::{Request, Multipart, Path as AxumPath,DefaultBodyLimit },
    middleware::Next,
    http::{header, HeaderMap, StatusCode},
    body::Body,
    routing::{post, get},
    Router,
    middleware,
};
use serde_json::json;
use thiserror::Error;
use tokio::{io::{AsyncWriteExt, AsyncReadExt, AsyncSeekExt}};
use std::path::Path;
use tokio_util::io::ReaderStream;
use http_range_header::parse_range_header;
use tower_http::services::ServeFile;
use tokio::fs::OpenOptions;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Fiel IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Multipart error: {0}")]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    #[error("Invalid API key")]
    AuthError,
    #[error("File not found")]
    NotFound,
    #[error("Invalid Range header")]
    InvalidRange,
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

// 将错误转换为HTTP响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::AuthError => (StatusCode::UNAUTHORIZED, "Unauthorized access"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "File not found"),
            AppError::InvalidRange => (StatusCode::RANGE_NOT_SATISFIABLE, "Invalid Range"),
            // 生产环境建议隐藏详细的系统错误信息
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal Server Error"),
        };
        let body = Json(json!({
            "error": error_message,
            "details": self.to_string(),
        }));

        (status, body).into_response()
    }
}

const API_KEY: &str = "secret-upload-key-123";

async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    match headers.get("X-API-Key") {
        Some(key) if key == API_KEY => Ok(next.run(request).await),
        _=>Err(StatusCode::UNAUTHORIZED),
    }
}

async fn upload_handler(
    headers: HeaderMap,
    mut multipart: Multipart
) -> Result<Json<serde_json::Value>, AppError> {
    // 确保上传目录存在
    let upload_dir = "uploads";
    tokio::fs::create_dir_all(upload_dir).await?;

    // 判断是否续传模式（前端会传X-Append: true）
    let is_append = headers.contains_key("X-Append");

    while let Some(field) = multipart.next_field().await? {
        let file_name = if let Some(name) = field.file_name() {
            name.split('/').last().unwrap_or("unnamed").to_string()
        } else {
            continue;
        };
        let file_path = Path::new(upload_dir).join(&file_name);

        // 配置打开模式
        let mut file = OpenOptions::new()
            .create(true) // 如果文件不存在则创建
            .write(true)  // 允许写入
            .truncate(!is_append) // 如果是续传，启用追加模式
            .open(&file_path) // 打开文件，如果不存在则创建
            .await?;

        // 使用 BufWriter 提高写入性能
        let mut writer = tokio::io::BufWriter::new(&mut file);
        // 核心：流式读取Field数据并写入文件
        // 这里的field本身实现了Stream，不会一次性加载到内存
        let mut field_stream = field;

        // 写入数据
        while let Some(chunk) = field_stream.chunk().await? {
            writer.write_all(&chunk).await?;
        }
        writer.flush().await?;

        println!("File saved: {:?}", file_path);
    }

    Ok(Json(json!({ "status": "ok", "message": "Upload successful" })))
}


async fn download_handler(
    headers: HeaderMap,
    AxumPath(filename): AxumPath<String>,
) -> Result<impl IntoResponse, AppError> {
    let file_path = std::path::Path::new("uploads").join(&filename);

    println!("Download request for: {:?}", &file_path);

    // 打开文件并获取元数据
    let mut file = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => return Err(AppError::NotFound),
    };
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    let mime_type = mime_guess::from_path(&filename).first_or_octet_stream();

    // 解析Range头
    let range_header = headers
        .get(header::RANGE)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| parse_range_header(h).ok());

    match range_header {
        // 场景1： 客户端请求了Range（断点续传/多线程下载）
        Some(ranges) => {
            // 这里为了简化，只处理第一个Range区间
            if let Some(range) = ranges.validate(file_size).ok().and_then(|v|v.first().cloned())
            {
                let start = range.start();
                // range.end是包含计算长度需要+1
                let end = range.end();
                let chunk_size = end - start + 1;

                // 移动文件指针
                file.seek(std::io::SeekFrom::Start(*start)).await?;

                // 限制读取长度，创建一个限制流
                // 注意：take接收的是u64
                let stream = ReaderStream::new(file.take(chunk_size));
                let body = Body::from_stream(stream);

                // 构造206 Partial Content响应
                let headers = [
                    (header::CONTENT_TYPE, mime_type.as_ref()),
                    (header::ACCEPT_RANGES, "bytes"),
                    (header::CONTENT_RANGE, &format!("bytes {}-{}/{}", start, end, file_size)),
                    (header::CONTENT_LENGTH, &chunk_size.to_string()),
                ];

                Ok((StatusCode::PARTIAL_CONTENT, headers, body).into_response())
            } else {
                Err(AppError::InvalidRange)
            }
        }
        // 场景2： 普通下载
        None => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            let headers = [
                (header::CONTENT_TYPE, mime_type.as_ref()),
                (header::ACCEPT_RANGES, "bytes"),
                (header::CONTENT_LENGTH, &file_size.to_string()),
            ];

            Ok((StatusCode::OK, headers, body).into_response())
        }
    }
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 之歌上传接口需要鉴权
    let protected_routes = Router::new()
        .route("/upload", post(upload_handler))
        .layer(middleware::from_fn(auth_middleware))
        .layer(DefaultBodyLimit::max(1000 * 1024 * 1024)); // 设置最大请求体为1000MB

    let public_routes = Router::new()
         .route_service("/", ServeFile::new("index.html"))
        // 下载路由：公开访问
        .route("/download/{filename}", get(download_handler));

    // 构建应用路由
    let app = Router::new()
    .merge(public_routes)
    .merge(protected_routes);

     // 绑定端口
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server listening on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await.unwrap();
}