use axum::{Router, routing::get};

use crate::{handlers::logs, state::AppState};

pub fn log_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(logs::search_logs))
        .route("/{id}", get(logs::get_log_by_id))
}
