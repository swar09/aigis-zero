use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::AuthUser,
    models::node::{IsolateNodeRequest, NodeFilterParams},
    state::AppState,
};

pub async fn list_nodes(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<NodeFilterParams>,
) -> Result<Json<Value>, AppError> {
    let (items, total) = state.node_service.list_nodes(params).await?;
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

pub async fn get_node_by_id(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let node_detail = state.node_service.get_node_by_id(id).await?;
    Ok(Json(json!({
        "success": true,
        "data": node_detail,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}

pub async fn isolate_node(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<IsolateNodeRequest>,
) -> Result<Json<Value>, AppError> {
    let res = state.node_service.isolate_node(id, payload.reason).await?;
    Ok(Json(json!({
        "success": true,
        "data": res,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}

pub async fn unisolate_node(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let res = state.node_service.unisolate_node(id).await?;
    Ok(Json(json!({
        "success": true,
        "data": res,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}
