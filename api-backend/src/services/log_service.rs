use std::sync::Arc;

use uuid::Uuid;

use crate::{
    error::AppError,
    models::log::{EventLogEntity, LogFilterParams},
    repositories::LogRepository,
};

#[derive(Clone)]
pub struct LogService {
    log_repo: Arc<dyn LogRepository>,
}

impl LogService {
    pub fn new(log_repo: Arc<dyn LogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn search_logs(
        &self,
        params: LogFilterParams,
    ) -> Result<Vec<EventLogEntity>, AppError> {
        self.log_repo.search_logs(params).await
    }

    pub async fn get_log_by_id(&self, event_id: Uuid) -> Result<EventLogEntity, AppError> {
        self.log_repo
            .find_by_id(event_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Log event with id '{event_id}' not found")))
    }
}
