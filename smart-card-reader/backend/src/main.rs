mod reader;
mod decoder;
mod server;

use log::info;
use tokio::sync::broadcast;
use std::sync::Arc;

use serde_json::json;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting Smart Card Reader Service...");

    // Channel for broadcasting card data to WebSockets
    // capacity 100
    let (tx, _rx) = broadcast::channel(100);

    // Spawn Reader Task
    let tx_reader = tx.clone();
    
    // We need to move reader into the task.
    // Initialize reader inside the task or move it.
    // Reader struct is not Send/Sync if it holds PCSC Context directly potentially (Context is Send? Yes usually).
    // Let's create it inside the task to be safe or investigate pcsc crate.
    // `pcsc::Context` is Send.
    
    tokio::spawn(async move {
        let mut reader = reader::CardReader::new().expect("Failed to initialize Reader");
        reader.run_monitor(move |data| {
            // Serialize data
            let msg = json!({
                "type": "CARD_INSERTED",
                "data": data
            }).to_string();
            
            if let Err(_e) = tx_reader.send(msg) {
                // error!("Failed to broadcast: {}", e);
            }
        }).await;
    });

    // Spawn Server
    // Initialize server
    let app_state = Arc::new(server::AppState { tx });
    
    use axum::{Router, routing::get};
    use std::net::SocketAddr;

    let app = Router::new()
        .route("/ws", get(server::ws_handler))
        .with_state(app_state);
        
    // CORS layer might be needed if frontend is on different port
    use tower_http::cors::{CorsLayer, Any};
    let app = app.layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8182));
    info!("WebSocket server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
