use axum:: {
    extract:: {
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};

use dashmap::DashMap;
use tokio::sync::broadcast;
use serde::Deserialize;
use tracing::{info};
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};

// 房间装填：每个房间ID对应一个广播通道的Sender
struct AppState {
    // Key: 房间名， value 广播发送端
    rooms: DashMap<String, broadcast::Sender<String>>,
}

#[derive(Deserialize)]
struct RoomParams {
    room: Option<String>,
}

#[tokio::main]
async fn main() {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter("axum_chat_v8=info").init();

    // 初始化共享状态
    let state = Arc::new(AppState {
        rooms: DashMap::new(),
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    info!("✅ 服务器运行中: ws://127.0.0.1:3000/ws");

    axum::serve(listener, app).await.unwrap();
}

// WebSocket 处理器
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<RoomParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let room_id = params.room.unwrap_or_else(|| "default".to_string());

    ws.on_upgrade(move |socket| handle_socket(socket, room_id, state))
}

async fn handle_socket(socket: WebSocket, room_id: String, state: Arc<AppState>) {
    // 分离发送和接收
    let (mut sink, mut stream) = socket.split();

    // 获取or创建房间的广播通道
    // 如果房间不存在则创建，通道容量设为100
    let tx = state
        .rooms
        .entry(room_id.clone())
        .or_insert_with(|| {
            info!("🏠 创建新房间: {}", room_id);
            let (tx, _rx) = broadcast::channel(100);
            tx
        })
        .clone();
    
    // 订阅该房间的消息
    let mut rx = tx.subscribe();
    info!("👤 用户进入房间: {}", room_id);

    // 创建两个并发任务处理收发

    // 任务1： 接收房间广播，发送给当前客户端
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break; // 客户端断开连接
            }
        }
    });

    // 任务2： 接收当前客户端发来的消息，广播给房间所有人
    let tx_clone = tx.clone();
    let room_id_clone = room_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Text(text) = msg {
                // 收到消息后，通过广播通道发送出去
                let _ = tx_clone.send(format!("[{}]: {}", room_id_clone, text));
            }
        }
    });
// e. 只要有一个任务结束（客户端断开），就清理资源
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("🚪 用户离开房间: {}", room_id);

    // f. 自动清理机制：如果房间没人了，删除该房间以释放内存
    if tx.receiver_count() == 0 {
        state.rooms.remove(&room_id);
        info!("🗑️ 房间 {} 已关闭", room_id);
    }
}