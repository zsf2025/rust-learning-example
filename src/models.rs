use serde::{Serialize, Deserialize};

// 定义我们的文章结构体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Article {
    pub id: usize,
    pub title: String,
    pub content: String
}

// 用于接收表单数据的结构体
// 字段名必须与HTML input的name属性一致
#[derive(Deserialize)]
pub struct CreateArticleForm {
    pub title: String,
    pub content: String,
}


