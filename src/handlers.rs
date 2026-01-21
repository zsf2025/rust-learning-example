use axum::{
    extract::{
        State,
        Query,
        Path
    }, 
    http::StatusCode, 
    Json
};
use sqlx::SqlitePool;
use crate::{
    error::AppError, 
    models::{
        CreateTodo, 
        Todo,
        Pagination,
        UpdateTodo
    },
};

// 共享状态
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub async fn create_todo(
    State(state): State<AppState>,
    Json(payload): Json<CreateTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    // title不为空检测
    if payload.title.trim().is_empty() {
        return Err(AppError::ValidationError("标题不能为空".to_string()));
    }

    // 插入数据库，同时返回创建好的任务数据
    let todo = sqlx::query_as::<_, Todo>(
        "INSERT INTO todos (title) VALUES (?) RETURNING id, title, done, created_at"
    )
    .bind(payload.title)
    // 查询并返回唯一一条结果，自动映射为目标类型。
    .fetch_one(&state.pool)
    .await?;

    // 日志记录创建任务
    tracing::info!("创建了一个新的任务:{}", todo.id);

    // 请求返回
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn list_todos(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>
) -> Result<Json<Vec<Todo>>, AppError> {
    let page = pagination.page.unwrap_or(1);
    let size = pagination.size.unwrap_or(10);
    let offset = (page - 1) * size;

    // 根据分页查询数据列表
    let todos = sqlx::query_as::<_, Todo>(
        "SELECT id, title, done, created_at FROM todos ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(size)
    .bind(offset)
    // 查询返回任务数量结果，没有数据返回空数组
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(todos))
}

pub async fn get_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Todo>, AppError> {
    let todo = sqlx::query_as::<_, Todo>(
        "SELECT id, title, done, created_at FROM todos WHERE id = ?"
    )
    .bind(id)
    // fetch_optional 返回 Option<T>，为空返回None
    .fetch_optional(&state.pool)
    .await?
    // 如果是None, 转换为AppError::NotFound
    .ok_or(AppError::NotFound)?;

    Ok(Json(todo))
}

pub async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateTodo>,
) -> Result<Json<Todo>, AppError> {
    // 首先检查是否存在，不存在直接抛出 404
    let _exists = sqlx::query("SELECT id FROM todos WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    // COALESCE(?, field) 表示：如果传入参数不为 NULL，用参数；否则保持原值
    let todo = sqlx::query_as::<_, Todo>(
        r#"
        UPDATE todos 
        SET title = COALESCE(?, title), 
            done = COALESCE(?, done)
        WHERE id = ? 
        RETURNING id, title, done, created_at
        "#
    )
    .bind(payload.title)
    .bind(payload.done)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(todo))
}

pub async fn delete_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, String), AppError> {
    let result = sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;

     // 无行受影响 →  todo 不存在 → 返回 404
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok((StatusCode::OK, "删除成功".to_string()))
}