#![forbid(unsafe_code)]
#![deny(warnings)]

mod app;
mod client;
mod models;
mod views;

use app::{AppState, build_router};
use client::StudioClient;
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gororoba_studio_web=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let backend_url =
        env::var("GOROROBA_BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8088".to_string());
    let host = env::var("GOROROBA_APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("GOROROBA_APP_PORT").unwrap_or_else(|_| "8090".to_string());
    let bind_addr = format!("{host}:{port}");

    let listener = TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind frontend listener on {bind_addr}: {err}"));

    tracing::info!(bind_addr, backend_url, "starting gororoba_studio_web");

    let state = AppState::new(StudioClient::new(backend_url));
    let router = build_router(state);

    axum::serve(listener, router)
        .await
        .unwrap_or_else(|err| panic!("frontend server failed: {err}"));
}
