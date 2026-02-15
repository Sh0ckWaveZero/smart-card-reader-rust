mod config;
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
    // Load configuration first (before logger init)
    let app_config = config::load();

    // Initialize logger with configured level
    std::env::set_var("RUST_LOG", &app_config.logging.level);
    env_logger::init();

    info!("Starting Smart Card Reader Service...");
    info!("Config: server={}", app_config.server);

    // Channel for UI updates (card events)
    let (tx_ui, rx_ui) = std::sync::mpsc::channel::<decoder::CardEvent>();

    // Clone config for background thread
    let server_config = app_config.server.clone();
    let output_config = app_config.output.clone();
    let card_config = app_config.card.clone();

    // Background thread for card reader + WebSocket server
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            // Channel for broadcasting card data to WebSockets
            let (tx_ws, _rx) = broadcast::channel::<String>(100);

            // Spawn WebSocket server
            let app_state = Arc::new(server::AppState {
                tx: tx_ws.clone(),
            });

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

            let addr = server_config.socket_addr();
            info!("WebSocket server listening on {addr}");

            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .expect("Failed to bind WebSocket server");

            tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app).await {
                    log::error!("WebSocket server error: {}", e);
                }
            });

            // Run card reader monitor with card config
            let mut card_reader =
                reader::CardReader::new(card_config).expect("Failed to initialize Card Reader");

            let output_config_clone = output_config.clone();
            card_reader
                .run_monitor(move |event| {
                    // Send to WebSocket clients with field mapping applied
                    let msg = match &event {
                        decoder::CardEvent::Inserted(data) => {
                            let mapped_data = decoder::apply_output_config(data, &output_config_clone);
                            json!({
                                "type": "CARD_INSERTED",
                                "data": mapped_data
                            })
                        }
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
    let ui_config = &app_config.ui;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([ui_config.window_width, ui_config.window_height])
            .with_min_inner_size([ui_config.min_width, ui_config.min_height])
            .with_title(&ui_config.window_title),
        ..Default::default()
    };

    let ws_url = app_config.server.websocket_url();
    let font_config = app_config.fonts.clone();

    if let Err(e) = eframe::run_native(
        &app_config.ui.window_title,
        options,
        Box::new(move |_cc| Ok(Box::new(ui::SmartCardApp::new(rx_ui, ws_url, font_config)))),
    ) {
        log::error!("Failed to run egui: {}", e);
    }
}
