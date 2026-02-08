use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use log::info;
use serde_json::json;
use crate::decoder::ThaiIDData;

pub struct AppState {
    pub tx: broadcast::Sender<String>,
}

#[allow(dead_code)]
pub async fn run_server(tx: broadcast::Sender<String>) {
    let app_state = Arc::new(AppState { tx });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    info!("WebSocket server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Err(_e) = socket.send(Message::Text(msg)).await {
            // client disconnected
            break;
        }
    }
}

#[allow(dead_code)]
pub fn create_event_json(event_type: &str, data: Option<ThaiIDData>) -> String {
    json!({
        "type": event_type,
        "data": data
    }).to_string()
}
