use std::sync::Arc;

use uuid::Uuid;

use crate::{
    error::AppError,
    models::alert::{AlertEntity, AlertFilterParams, UpdateAlertStatusRequest, UpdateAlertStatusResponse},
    repositories::AlertRepository,
};

#[derive(Clone)]
pub struct AlertService {
    alert_repo: Arc<dyn AlertRepository>,
}

impl AlertService {
    pub fn new(alert_repo: Arc<dyn AlertRepository>) -> Self {
        Self { alert_repo }
    }

    pub async fn list_alerts(&self, params: AlertFilterParams) -> Result<(Vec<AlertEntity>, i64), AppError> {
        self.alert_repo.find_all(params).await
    }

    pub async fn get_alert_by_id(&self, alert_id: Uuid) -> Result<AlertEntity, AppError> {
        self.alert_repo
            .find_by_id(alert_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Alert with id '{alert_id}' not found")))
    }

    pub async fn update_status(
        &self,
        alert_id: Uuid,
        req: UpdateAlertStatusRequest,
    ) -> Result<UpdateAlertStatusResponse, AppError> {
        self.alert_repo.update_status(alert_id, req).await
    }
}
