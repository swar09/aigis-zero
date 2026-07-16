use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::AuthUser,
    models::alert::{AlertFilterParams, UpdateAlertStatusRequest},
    state::AppState,
};

pub async fn list_alerts(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<AlertFilterParams>,
) -> Result<Json<Value>, AppError> {
    let (items, total) = state.alert_service.list_alerts(params).await?;
    Ok(Json(json!({
        "success": true,
        "data": {
            "total": total,
            "items": items
        },
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}

pub async fn get_alert_by_id(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let alert = state.alert_service.get_alert_by_id(id).await?;
    Ok(Json(json!({
        "success": true,
        "data": alert,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}

pub async fn update_alert_status(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAlertStatusRequest>,
) -> Result<Json<Value>, AppError> {
    let res = state.alert_service.update_status(id, payload).await?;
    Ok(Json(json!({
        "success": true,
        "data": res,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}
