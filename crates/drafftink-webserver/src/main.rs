//! DrafftInk Web Server
//!
//! A self-contained binary that embeds the web frontend and serves it
//! alongside the WebSocket relay. Zero external dependencies for deployment.

use axum::{
    Router,
    extract::Request,
    response::{IntoResponse, Response},
    routing::get,
};
use drafftink_server::AppState;
use rust_embed::Embed;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Embed)]
#[folder = "../../web/"]
struct Assets;

fn print_usage() {
    eprintln!("Usage: drafftink-webserver [OPTIONS]");
    eprintln!();
    eprintln!("A self-contained web server for DrafftInk with embedded frontend.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --port <PORT>        Port to listen on (default: 8080)");
    eprintln!("  --address <ADDRESS>  Address to bind to (default: 127.0.0.1)");
    eprintln!("  --cors               Enable permissive CORS headers");
    eprintln!("  --help               Show this help message");
}

struct Config {
    port: u16,
    address: String,
    cors: bool,
}

fn parse_args() -> Config {
    let mut config = Config {
        port: 8080,
        address: "127.0.0.1".to_string(),
        cors: false,
    };

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "--port" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --port requires a value");
                    std::process::exit(1);
                }
                config.port = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: invalid port number '{}'", args[i]);
                    std::process::exit(1);
                });
            }
            "--address" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --address requires a value");
                    std::process::exit(1);
                }
                config.address = args[i].clone();
            }
            "--cors" => {
                config.cors = true;
            }
            other => {
                eprintln!("Error: unknown option '{}'", other);
                eprintln!();
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    config
}

async fn static_handler(req: Request) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [("content-type", mime.as_ref())],
                file.data.into_owned(),
            )
                .into_response()
        }
        None => {
            // Try index.html as fallback for SPA routing
            match Assets::get("index.html") {
                Some(file) => {
                    (
                        [("content-type", "text/html")],
                        file.data.into_owned(),
                    )
                        .into_response()
                }
                None => (
                    axum::http::StatusCode::NOT_FOUND,
                    "Not Found",
                )
                    .into_response(),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let config = parse_args();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "drafftink_webserver=info,drafftink_server=info,tower_http=info".into()),
        )
        .init();

    let state = Arc::new(AppState::new());

    let mut app = Router::new()
        .route("/ws", get(drafftink_server::ws_handler))
        .route("/health", get(drafftink_server::health))
        .fallback(static_handler)
        .with_state(state);

    if config.cors {
        app = app.layer(CorsLayer::permissive());
    }

    let addr: SocketAddr = format!("{}:{}", config.address, config.port)
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("Error: invalid address '{}:{}': {}", config.address, config.port, e);
            std::process::exit(1);
        });

    info!("DrafftInk web server listening on http://{}", addr);
    info!("WebSocket endpoint: ws://{}/ws", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| {
        eprintln!("Error: failed to bind to {}: {}", addr, e);
        std::process::exit(1);
    });
    axum::serve(listener, app).await.unwrap();
}
