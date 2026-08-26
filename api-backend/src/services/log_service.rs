//! Business logic service for searching and retrieving historical telemetry logs.

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    error::AppError,
    models::log::{EventLogEntity, LogFilterParams},
    repositories::LogRepository,
};

/// Service handling telemetry event log queries and detail views.
#[derive(Clone)]
pub struct LogService {
    log_repo: Arc<dyn LogRepository>,
}

impl LogService {
    /// Creates a new `LogService` with the injected repository.
    pub fn new(log_repo: Arc<dyn LogRepository>) -> Self {
        Self { log_repo }
    }

    /// Searches telemetry event logs matching the given filter criteria.
    pub async fn search_logs(&self, params: LogFilterParams) -> Result<Vec<EventLogEntity>, AppError> {
        self.log_repo.search_logs(params).await
    }

    /// Retrieves a single telemetry event by its unique UUID.
    pub async fn get_log_by_id(&self, event_id: Uuid) -> Result<EventLogEntity, AppError> {
        self.log_repo
            .find_by_id(event_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Log event with id '{event_id}' not found")))
    }
}
