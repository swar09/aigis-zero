use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::{
    error::AppError,
    middleware::AuthUser,
    models::auth::{LoginRequest, LoginResponse},
    state::AppState,
};

pub async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> Result<Json<Value>, AppError> {
    let res: LoginResponse = state.auth_service.login(payload).await?;
    Ok(Json(json!({
        "success": true,
        "data": res,
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}

pub async fn me(AuthUser(claims): AuthUser) -> Result<Json<Value>, AppError> {
    Ok(Json(json!({
        "success": true,
        "data": {
            "username": claims.sub,
            "role": claims.role,
            "permissions": [
                "nodes:read",
                "nodes:isolate",
                "alerts:read",
                "alerts:write",
                "logs:read"
            ]
        },
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    })))
}
