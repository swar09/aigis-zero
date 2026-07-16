use axum::{Router, routing::get};

use crate::{handlers::health, state::AppState};

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
}
