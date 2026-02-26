use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::audit_log::AuditLogger;
use crate::config::SecurityConfig;
use crate::rate_limiter::RateLimiter;

pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub security: SecurityConfig,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub audit_logger: Arc<AuditLogger>,
}


pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let client_ip = addr.ip();

    // Check rate limit if enabled
    if let Some(ref rate_limiter) = state.rate_limiter {
        // Check request rate limit
        if !rate_limiter.check_request(client_ip) {
            log::warn!("⚠️ Rate limit exceeded for {}", client_ip);
            state.audit_logger.log_rate_limit(client_ip, "request");
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many requests. Please try again later.",
            )
                .into_response();
        }

        // Check connection limit
        if !rate_limiter.check_connection(client_ip) {
            log::warn!("⚠️ Connection limit exceeded for {}", client_ip);
            state.audit_logger.log_rate_limit(client_ip, "connection");
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many concurrent connections. Please close existing connections.",
            )
                .into_response();
        }
    }

    // Check authentication if enabled
    if state.security.enable_authentication {
        let api_key = headers
            .get(&state.security.api_key_header)
            .and_then(|v| v.to_str().ok());

        match api_key {
            Some(key) if state.security.is_valid_key(key) => {
                log::debug!("✓ Authentication successful");
                // Log authentication success with first 4 chars of key as hint
                let key_hint = if key.len() >= 4 {
                    Some(&key[..4])
                } else {
                    Some(key)
                };
                state.audit_logger.log_auth_success(client_ip, key_hint);
            }
            Some(_) => {
                log::warn!("⚠️ Invalid API key provided");
                state.audit_logger.log_auth_failure(client_ip, "Invalid API key");
                return (
                    StatusCode::UNAUTHORIZED,
                    "Invalid API key. Provide a valid X-API-Key header.",
                )
                    .into_response();
            }
            None => {
                log::warn!("⚠️ No API key provided");
                state.audit_logger.log_auth_failure(client_ip, "No API key provided");
                return (
                    StatusCode::UNAUTHORIZED,
                    format!("Authentication required. Provide {} header.", state.security.api_key_header),
                )
                    .into_response();
            }
        }
    }

    // Log connection opened
    state.audit_logger.log_connection_open(client_ip);

    ws.on_upgrade(move |socket| handle_socket(socket, state, client_ip))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, client_ip: std::net::IpAddr) {
    let connection_start = std::time::Instant::now();
    let mut rx = state.tx.subscribe();

    // Handle WebSocket messages
    while let Ok(msg) = rx.recv().await {
        if let Err(_e) = socket.send(Message::Text(msg)).await {
            // client disconnected
            break;
        }
    }

    // Calculate connection duration
    let duration_ms = connection_start.elapsed().as_millis() as u64;

    // Release connection slot when client disconnects
    if let Some(ref rate_limiter) = state.rate_limiter {
        rate_limiter.release_connection(client_ip);
        log::debug!("✓ Connection released for {}", client_ip);
    }

    // Log connection closed
    state.audit_logger.log_connection_close(client_ip, Some(duration_ms));
}
