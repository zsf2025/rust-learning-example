use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use tracing;

#[derive(Debug)]
pub enum AppError {
    Sqlx(sqlx::Error),      // 数据库错误
    NotFound,               // 资源未找到错误
    ValidationError(String),// 验证错误
}

// From trait是【类型转换】的标准接口，实现From<A> for B后，
// 可以通过?操作符将A类型自动转换为B类型
// 允许直接使用?将sqlx错误转换为AppError
impl From<sqlx::Error> for AppError {
    fn from(inner: sqlx::Error) -> Self {
        AppError::Sqlx(inner)
    }
}

// Axum要求路由handler的返回值必须实现IntoResponse trait, 这里实现该trait
// 让AppError能自动转换为标准HTTP响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            // 处理数据库错误
            AppError::Sqlx(e) => {
                // 记录详细的日志到服务器
                tracing::error!("Database error:{:?}", e);
                // 避免暴露数据库地址、SQL语句、表结构等敏感信息，这里只返回【服务器错误】
                (StatusCode::INTERNAL_SERVER_ERROR, "服务器错误".to_string())
            },
            // 处理资源未找到
            AppError::NotFound => (StatusCode::NOT_FOUND, "任务不存在".to_string()),
            // 处理验证错误
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(json!({
            "error": msg
        }));

        (status, body).into_response()
    }
}