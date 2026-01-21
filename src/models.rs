use serde::{Serialize, Deserialize};
use chrono;
use sqlx::{FromRow};

// Debug（调试打印）自动生成打印结构体所有字段的逻辑
// Serialize（JSON序列化）自动将结构体字段序列化为 JSON 键值对
// FromRow（数据库行映射）自动将数据库查询结果的列与结构体字段一一映射
#[derive(Debug, Serialize, FromRow)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub done: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// 自动实现Debug
// Deserialize JSON反序列化
#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

// 用于分页参数
#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<u32>,
    pub size: Option<u32>,
}

// 更新任务时的请求体 (DTO)
#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub done: Option<bool>,
}