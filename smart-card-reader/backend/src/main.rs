mod decoder;
mod reader;
mod server;
mod ui;

use axum::{routing::get, Router};
use log::info;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;

fn main() {
    env_logger::init();
    info!("Starting Smart Card Reader Service...");

    // Channel for UI updates (card events)
    let (tx_ui, rx_ui) = std::sync::mpsc::channel::<decoder::CardEvent>();

    // Background thread for card reader + WebSocket server
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            // Channel for broadcasting card data to WebSockets
            let (tx_ws, _rx) = broadcast::channel::<String>(100);

            // Spawn WebSocket server
            let app_state = Arc::new(server::AppState { tx: tx_ws.clone() });

            use tower_http::cors::{Any, CorsLayer};
            let app = Router::new()
                .route("/ws", get(server::ws_handler))
                .with_state(app_state)
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                );

            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8182));
            info!("WebSocket server listening on {}", addr);

            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .expect("Failed to bind WebSocket server");

            tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app).await {
                    log::error!("WebSocket server error: {}", e);
                }
            });

            // Run card reader monitor
            let mut card_reader =
                reader::CardReader::new().expect("Failed to initialize Card Reader");

            card_reader
                .run_monitor(move |event| {
                    // Send to WebSocket clients
                    let msg = match &event {
                        decoder::CardEvent::Inserted(data) => json!({
                            "type": "CARD_INSERTED",
                            "data": data
                        }),
                        decoder::CardEvent::Removed => json!({
                            "type": "CARD_REMOVED"
                        }),
                    }
                    .to_string();

                    if let Err(e) = tx_ws.send(msg) {
                        log::debug!("No WebSocket clients connected: {}", e);
                    }

                    // Send to UI
                    if let Err(e) = tx_ui.send(event) {
                        log::error!("Failed to send to UI: {}", e);
                    }
                })
                .await;
        });
    });

    // Run egui on main thread (required by most platforms)
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Thai Smart Card Reader"),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "Thai Smart Card Reader",
        options,
        Box::new(|_cc| Ok(Box::new(ui::SmartCardApp::new(rx_ui)))),
    ) {
        log::error!("Failed to run egui: {}", e);
    }
}
