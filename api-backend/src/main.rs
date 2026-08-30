//! Application binary entry point for the Aigis-Zero EDR API Backend server.

use std::net::SocketAddr;

use edr_api_backend::{config::Settings, kafka, routes, state::AppState};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,edr_api_backend=debug".into()),
        )
        .json()
        .init();

    info!("Starting Aigis-Zero EDR API Backend...");

    let settings = Settings::load_from_env()?;
    let shutdown_token = CancellationToken::new();

    let state = AppState::new(settings.clone())?;

    // Spawn Background Kafka Consumer
    let kafka_brokers = settings.kafka_brokers.clone();
    let kafka_group = settings.kafka_consumer_group.clone();
    let broadcast_tx = state.broadcast_tx.clone();
    let kafka_shutdown = shutdown_token.clone();

    tokio::spawn(async move {
        kafka::start_kafka_consumer(
            &kafka_brokers,
            &kafka_group,
            &[
                "aigis.events.raw",
                "aigis.events.norm",
                "aigis.events.process",
                "aigis.events.network",
                "aigis.events.file",
                "aigis.events.auth",
                "aigis.alerts",
                "aigis.health",
            ],
            broadcast_tx,
            kafka_shutdown,
        )
        .await;
    });

    // Build Axum Router with standard middleware and security headers
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = routes::create_router(state)
        .layer(axum::middleware::from_fn(add_security_headers))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let bind_addr: SocketAddr = format!("{}:{}", settings.host, settings.port).parse()?;
    let listener = TcpListener::bind(bind_addr).await?;

    info!(
        host = %settings.host,
        port = %settings.port,
        "Aigis-Zero API Backend listening"
    );

    let shutdown_signal = shutdown_token.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Received shutdown signal, stopping API server and background workers...");
            shutdown_signal.cancel();
        })
        .await?;

    info!("Aigis-Zero API Backend shutdown complete.");
    Ok(())
}

async fn add_security_headers(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        axum::http::HeaderName::from_static("x-xss-protection"),
        axum::http::HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        axum::http::HeaderName::from_static("referrer-policy"),
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}
