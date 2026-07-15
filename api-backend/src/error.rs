use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use diesel_async::pooled_connection::deadpool::PoolError;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Kafka error: {0}")]
    KafkaError(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound(_) => "RESOURCE_NOT_FOUND",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::Conflict(_) => "CONFLICT",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::KafkaError(_) => "KAFKA_ERROR",
            Self::ExternalServiceError(_) => "EXTERNAL_SERVICE_ERROR",
            Self::InternalServerError(_) => "INTERNAL_SERVER_ERROR",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::KafkaError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(json!({
            "success": false,
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "details": null
            },
            "meta": {
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }));

        (status, body).into_response()
    }
}

impl From<PoolError> for AppError {
    fn from(err: PoolError) -> Self {
        Self::DatabaseError(format!("Connection pool error: {err}"))
    }
}

impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => Self::NotFound("Requested record not found".into()),
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                info,
            ) => Self::Conflict(info.message().to_string()),
            other => Self::DatabaseError(other.to_string()),
        }
    }
}
