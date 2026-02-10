//! DrafftInk WebSocket Relay Server
//!
//! A simple relay server that broadcasts CRDT updates between clients in the same room.

use axum::{Router, routing::get};
use drafftink_server::{AppState, health, ws_handler};
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing::info;

/// Index page
async fn index() -> &'static str {
    "DrafftInk Relay Server - Connect via WebSocket at /ws"
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "drafftink_server=info,tower_http=info".into()),
        )
        .init();

    let state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(ws_handler))
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));
    info!("DrafftInk relay server listening on {}", addr);
    info!("WebSocket endpoint: ws://localhost:3030/ws");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
