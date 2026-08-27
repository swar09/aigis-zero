use std::sync::Arc;

use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use tracing::error;

use crate::{db::DbPool, metrics::EngineMetrics};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub metrics: Arc<EngineMetrics>,
}

pub fn create_health_router(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(liveness_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

async fn liveness_handler() -> &'static str {
    "OK"
}

async fn readiness_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.pool.get().await {
        Ok(_) => (StatusCode::OK, "READY"),
        Err(e) => {
            error!(error = %e, "Database pool readiness check failed");
            (StatusCode::SERVICE_UNAVAILABLE, "DATABASE_UNAVAILABLE")
        }
    }
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.metrics.format_prometheus()
}
