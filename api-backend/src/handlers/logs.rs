use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{error::AppError, middleware::AuthUser, models::log::LogFilterParams, state::AppState};

pub async fn search_logs(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<LogFilterParams>,
) -> Result<Json<Value>, AppError> {
    let items = state.log_service.search_logs(params).await?;
    Ok(Json(json!({
        "success": true,
        "data": {
            "items": items
        },
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}

pub async fn get_log_by_id(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let log_event = state.log_service.get_log_by_id(id).await?;
    Ok(Json(json!({
        "success": true,
        "data": log_event,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}
