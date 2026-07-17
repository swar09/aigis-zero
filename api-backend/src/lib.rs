//! # edr-api-backend
//!
//! High-throughput cybersecurity operator gateway and REST/WebSocket API server for the **Aigis-Zero EDR** ecosystem.
//!
//! `edr-api-backend` bridges SOC operator dashboards with distributed endpoint telemetry,
//! real-time Kafka event streams, and PostgreSQL databases using non-blocking asynchronous I/O.
//!
//! # Architecture Overview
//!
//! The backend is organized into decoupled layers:
//! - **Transport & Middleware (`routes/`, `middleware/`)**: Axum 0.8 HTTP routers, JWT authentication guards, and request tracing.
//! - **Controllers (`handlers/`)**: Deserializes JSON requests and query parameters, dispatches work to services, and formats standard responses.
//! - **Business Logic (`services/`)**: Pure async business rules, coordinating database repositories and gRPC command dispatches.
//! - **Data Access (`repositories/`)**: Non-blocking database access powered by [`diesel_async`] and [`deadpool_diesel`].
//! - **Real-Time Streaming (`kafka/`, `handlers::ws`)**: Background Kafka stream consumer piping live telemetry directly into [`tokio::sync::broadcast`] channels for instant WebSocket fanout.
//!
//! # Quick Start Example
//!
//! ```no_run
//! use edr_api_backend::config::Settings;
//! use edr_api_backend::state::AppState;
//! use edr_api_backend::routes::create_router;
//! use std::net::SocketAddr;
//! use tokio::net::TcpListener;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let settings = Settings::load_from_env()?;
//!     let state = AppState::new(settings.clone())?;
//!     let app = create_router(state);
//!
//!     let addr: SocketAddr = format!("{}:{}", settings.host, settings.port).parse()?;
//!     let listener = TcpListener::bind(addr).await?;
//!     axum::serve(listener, app).await?;
//!     Ok(())
//! }
//! ```

pub mod clients;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod kafka;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;

pub use config::Settings;
pub use error::AppError;
pub use state::AppState;
