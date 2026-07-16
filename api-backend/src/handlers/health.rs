use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::state::AppState;

pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "healthy" })))
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let nodes_status = match state.nodes_pool.get().await {
        Ok(_) => "up",
        Err(_) => "down",
    };

    let alerts_status = match state.alerts_pool.get().await {
        Ok(_) => "up",
        Err(_) => "down",
    };

    let logs_status = match state.logs_pool.get().await {
        Ok(_) => "up",
        Err(_) => "down",
    };

    let is_ready = nodes_status == "up" && alerts_status == "up" && logs_status == "up";

    let status_code = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(json!({
            "status": if is_ready { "ready" } else { "degraded" },
            "services": {
                "nodes_db": nodes_status,
                "alerts_db": alerts_status,
                "logs_db": logs_status
            }
        })),
    )
}
