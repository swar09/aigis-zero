use axum::{
    Router,
    routing::{get, patch},
};

use crate::{handlers::alerts, state::AppState};

pub fn alert_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(alerts::list_alerts))
        .route("/{id}", get(alerts::get_alert_by_id))
        .route("/{id}/status", patch(alerts::update_alert_status))
}
