mod audit_log;
mod config;
mod crypto;
mod decoder;
mod rate_limiter;
mod reader;
mod server;
mod ui;
mod validation;

use axum::{routing::get, Router};
use log::info;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;

// TLS/SSL imports (only axum_server needed for RustlsConfig)

/// Load TLS configuration from certificate and key files
///
/// # Returns
/// Returns `axum_server::tls_rustls::RustlsConfig` configured with the provided certificates
///
/// # Errors
/// Returns error if certificate or key files cannot be read or parsed
async fn load_tls_config(cert_path: &str, key_path: &str) -> anyhow::Result<axum_server::tls_rustls::RustlsConfig> {
    // Use axum_server's RustlsConfig for easier integration
    let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load TLS config: {}", e))?;

    Ok(config)
}

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
            let security_config = app_config.security.clone();

            // Initialize rate limiter if enabled
            let rate_limiter = if security_config.enable_rate_limiting {
                let config = rate_limiter::RateLimitConfig {
                    max_requests: security_config.rate_limit_requests,
                    window: std::time::Duration::from_secs(security_config.rate_limit_window_secs),
                    max_connections: security_config.rate_limit_max_connections,
                };
                info!("🚦 Rate limiting ENABLED:");
                info!("   Max requests: {} per {} seconds", config.max_requests, config.window.as_secs());
                info!("   Max connections: {} per IP", config.max_connections);
                let limiter = Arc::new(rate_limiter::RateLimiter::new(config));

                // Spawn cleanup task
                let limiter_clone = limiter.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
                    loop {
                        interval.tick().await;
                        limiter_clone.cleanup(std::time::Duration::from_secs(600)); // cleanup entries older than 10 minutes
                        let stats = limiter_clone.get_stats();
                        log::debug!("🚦 Rate limiter stats: {} tracked IPs, {} active connections",
                            stats.tracked_ips, stats.total_active_connections);
                    }
                });

                Some(limiter)
            } else {
                log::warn!("⚠️ Rate limiting DISABLED - Vulnerable to DoS attacks!");
                None
            };

            // Initialize audit logger
            let audit_logger = Arc::new(audit_log::AuditLogger::new(
                security_config.enable_audit_logging
            ));

            let app_state = Arc::new(server::AppState {
                tx: tx_ws.clone(),
                security: security_config.clone(),
                rate_limiter,
                audit_logger: audit_logger.clone(),
            });

            // Log security status
            if security_config.enable_authentication {
                let key_count = security_config.get_api_keys().len();
                if key_count > 0 {
                    info!("🔐 WebSocket authentication ENABLED ({} API keys configured)", key_count);
                } else {
                    log::error!("❌ Authentication enabled but NO API keys configured!");
                }
            } else {
                log::warn!("⚠️ WebSocket authentication DISABLED - Anyone can connect!");
            }

            // Initialize encryption service if enabled
            let crypto_service = if security_config.enable_encryption {
                match crypto::CryptoService::from_env() {
                    Ok(service) => {
                        let field_count = security_config.encrypted_fields.len();
                        info!("🔒 PII encryption ENABLED ({} fields protected)", field_count);
                        info!("   Encrypted fields: {:?}", security_config.encrypted_fields);
                        Some(Arc::new(service))
                    }
                    Err(e) => {
                        log::error!("❌ Encryption enabled but failed to initialize: {}", e);
                        log::error!("   Set ENCRYPTION_KEY environment variable:");
                        log::error!("   export ENCRYPTION_KEY=$(openssl rand -base64 32)");
                        panic!("Encryption configuration error");
                    }
                }
            } else {
                log::warn!("⚠️ PII encryption DISABLED - Sensitive data transmitted in plaintext!");
                None
            };

            use tower_http::cors::{Any, CorsLayer};

            // Configure CORS based on settings
            let cors_layer = if server_config.cors_allow_all {
                log::warn!("⚠️ CORS allow_all is ENABLED - This is INSECURE for production!");
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            } else {
                let allowed_origins = server_config.get_allowed_origins();
                if allowed_origins.is_empty() {
                    log::error!("❌ CORS restricted mode enabled but NO allowed origins configured!");
                    log::error!("   Set ALLOWED_ORIGINS env var or add to config.toml");
                } else {
                    log::info!("✓ CORS restricted to allowed origins: {:?}", allowed_origins);
                }

                let origins: Vec<_> = allowed_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok())
                    .collect();

                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods([axum::http::Method::GET])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                    ])
            };

            let app = Router::new()
                .route("/", get(server::ws_handler))
                .with_state(app_state)
                .layer(cors_layer);

            let addr = server_config.socket_addr();

            // Start server with or without TLS
            if server_config.enable_tls {
                info!("🔒 Starting HTTPS WebSocket server (wss://) on {addr}");

                // Load TLS configuration
                let tls_config = match load_tls_config(&server_config.tls_cert_path, &server_config.tls_key_path).await {
                    Ok(config) => config,
                    Err(e) => {
                        log::error!("❌ Failed to load TLS config: {}", e);
                        log::error!("   Cert: {}", server_config.tls_cert_path);
                        log::error!("   Key: {}", server_config.tls_key_path);
                        panic!("TLS configuration error");
                    }
                };

                tokio::spawn(async move {
                    if let Err(e) = axum_server::bind_rustls(addr, tls_config)
                        .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                        .await
                    {
                        log::error!("WebSocket server error: {}", e);
                    }
                });
            } else {
                info!("WebSocket server listening on {addr}");
                log::warn!("⚠️ TLS is DISABLED - Communication is NOT encrypted!");

                let listener = tokio::net::TcpListener::bind(addr)
                    .await
                    .expect("Failed to bind WebSocket server");

                tokio::spawn(async move {
                    if let Err(e) = axum::serve(
                        listener,
                        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                    )
                    .await
                    {
                        log::error!("WebSocket server error: {}", e);
                    }
                });
            }

            // Run card reader monitor with card config
            let mut card_reader =
                reader::CardReader::new(card_config).expect("Failed to initialize Card Reader");

            let output_config_clone = output_config.clone();
            let security_config_clone = security_config.clone();
            let audit_logger_clone = audit_logger.clone();
            card_reader
                .run_monitor(move |event| {
                    // Send to WebSocket clients with field mapping applied
                    let msg = match &event {
                        decoder::CardEvent::Inserted(data) => {
                            // Validate card data
                            let thai_name = format!("{} {} {} {}", data.th_prefix, data.th_firstname, data.th_middlename, data.th_lastname);
                            let validation_errors = crate::validation::CardDataValidator::validate_all(
                                Some(&data.citizen_id),
                                Some(&data.birthday),
                                Some(&data.issue_date),
                                Some(&data.expire_date),
                                Some(&data.sex),
                                Some(&thai_name),
                                Some(&data.full_name_en),
                                Some(&data.address),
                            );

                            let mut has_security_threat = false;

                            for (field, err) in validation_errors {
                                let (err_type, details, is_security) = match err {
                                    crate::validation::ValidationError::Format(msg) => ("Format", msg, false),
                                    crate::validation::ValidationError::Integrity(msg) => ("Integrity", msg, false),
                                    crate::validation::ValidationError::Security(msg) => ("Security", msg, true),
                                };

                                if is_security {
                                    has_security_threat = true;
                                }

                                audit_logger_clone.log_validation_failure(
                                    None,
                                    &field,
                                    err_type,
                                    &details,
                                    is_security,
                                );
                            }

                            if has_security_threat {
                                log::error!("❌ Card data contains security threats. Payload rejected.");
                                return; // Abort processing and do not broadcast
                            }

                            let mapped_data = decoder::apply_output_config(data, &output_config_clone);
                            // Flatten mapped_data into the top-level object alongside "mode"
                            let mut obj = serde_json::Map::new();
                            obj.insert("mode".to_string(), json!("readsmartcard"));
                            if let serde_json::Value::Object(fields) = mapped_data {
                                for (k, v) in fields {
                                    // Encrypt sensitive fields if encryption is enabled
                                    let final_value = if security_config_clone.should_encrypt_field(&k) {
                                        if let Some(ref crypto) = crypto_service {
                                            if let Some(plaintext) = v.as_str() {
                                                match crypto.encrypt_to_base64(plaintext) {
                                                    Ok(encrypted) => {
                                                        log::debug!("🔒 Encrypted field: {}", k);
                                                        json!(encrypted)
                                                    }
                                                    Err(e) => {
                                                        log::error!("❌ Failed to encrypt field '{}': {}", k, e);
                                                        v // Keep original value on encryption failure
                                                    }
                                                }
                                            } else {
                                                v // Non-string value, keep original
                                            }
                                        } else {
                                            v // No crypto service available
                                        }
                                    } else {
                                        v // Field not in encrypted list
                                    };
                                    obj.insert(k, final_value);
                                }
                            }
                            serde_json::Value::Object(obj)
                        }
                        decoder::CardEvent::Removed => json!({
                            "mode": "removedsmartcard"
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
            .with_max_inner_size([ui_config.max_width, ui_config.max_height])
            .with_resizable(false)
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
