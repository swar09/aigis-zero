use std::sync::Arc;

use axum::{Router, extract::State, response::IntoResponse, routing::get};

use crate::metrics::PipelineMetrics;

#[derive(Clone)]
pub struct PipelineHealthState {
    pub metrics: Arc<PipelineMetrics>,
}

/// Builds the Axum HTTP router for health probes and Prometheus metrics.
pub fn create_health_router(state: PipelineHealthState) -> Router {
    Router::new()
        .route("/health/live", get(liveness_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/healthz", get(liveness_handler))
        .route("/readyz", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

async fn liveness_handler() -> &'static str {
    "OK"
}

async fn readiness_handler() -> &'static str {
    "READY"
}

async fn metrics_handler(State(state): State<PipelineHealthState>) -> impl IntoResponse {
    state.metrics.format_prometheus()
}
