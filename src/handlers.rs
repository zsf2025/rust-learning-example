use axum::{
    extract::{
        State,
        Path
    },
    response::{Html, IntoResponse, Redirect},
    Form
};
use crate::{
    models::{
        Article,
        CreateArticleForm
    },
};
use tera::{Context, Tera};

use std:: {
    sync:: {Arc, RwLock }
};

// 共享状态
pub struct AppState {
    pub tera: Tera,
    pub posts: RwLock<Vec<Article>>,
}

// 首页：渲染文章列表
pub async fn home_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // 获取读锁， 读取数据
    let posts = state.posts.read().unwrap();

    // 创建 Tera 上下文并注入变量
    let mut context = Context::new();
    context.insert("posts", &*posts);

    // 渲染模板
    match state.tera.render("index.html", &context) {
        Ok(html) => Html(html),
        Err(err) => {
            eprintln!("模板渲染错误: {}", err);
            Html("<h1>模板渲染错误</h1>".to_string())
        }
    }
}

// 创建文章: 处理表单提交
pub async fn create_post_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CreateArticleForm>
) -> impl IntoResponse {
    // 获取写锁， 以便修改数据
    let mut posts = state.posts.write().unwrap();

    let new_id = posts.len() + 1;
    let new_post = Article {
        id: new_id,
        title: form.title,
        content: form.content
    };
    posts.push(new_post);

    // 重定向回首页，这就是所谓的PRG模式（Post/Redirect/Get）
    Redirect::to("/")
}

// 文章详情：根据ID渲染
pub async fn post_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<usize>
) -> impl IntoResponse {
    // 获取读锁， 读取数据
    let posts = state.posts.read().unwrap();

    // 根据ID查找文章
    if let Some(post) = posts.iter().find(|p| p.id == id) {
        let mut context = Context::new();
        context.insert("post", post);
        let html = state.tera.render("post.html", &context).unwrap();
        Html(html)
    } else {
        Html("<h1>文章未找到</h1>".to_string())
    }
}