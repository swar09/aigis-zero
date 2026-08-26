use async_trait::async_trait;
use chrono::Utc;
use diesel::{dsl::count_star, prelude::*};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    db::{DbPool, schema::alerts},
    error::AppError,
    models::alert::{AlertEntity, AlertFilterParams, UpdateAlertStatusRequest, UpdateAlertStatusResponse},
};

#[async_trait]
pub trait AlertRepository: Send + Sync {
    async fn find_all(&self, params: AlertFilterParams) -> Result<(Vec<AlertEntity>, i64), AppError>;
    async fn find_by_id(&self, target_id: Uuid) -> Result<Option<AlertEntity>, AppError>;
    async fn update_status(
        &self,
        target_id: Uuid,
        req: UpdateAlertStatusRequest,
    ) -> Result<UpdateAlertStatusResponse, AppError>;
}

#[derive(Clone)]
pub struct DieselAlertRepository {
    pool: DbPool,
}

impl DieselAlertRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AlertRepository for DieselAlertRepository {
    async fn find_all(&self, params: AlertFilterParams) -> Result<(Vec<AlertEntity>, i64), AppError> {
        let mut conn = self.pool.get().await?;

        let mut query = alerts::table.into_boxed();
        let mut count_query = alerts::table.into_boxed();

        if let Some(ref sev) = params.severity {
            query = query.filter(alerts::severity.eq(sev));
            count_query = count_query.filter(alerts::severity.eq(sev));
        }
        if let Some(ref st) = params.status {
            query = query.filter(alerts::status.eq(st));
            count_query = count_query.filter(alerts::status.eq(st));
        }
        if let Some(nid) = params.node_id {
            query = query.filter(alerts::node_id.eq(nid));
            count_query = count_query.filter(alerts::node_id.eq(nid));
        }
        if let Some(ref tech) = params.mitre_technique {
            query = query.filter(alerts::mitre_technique_id.eq(tech));
            count_query = count_query.filter(alerts::mitre_technique_id.eq(tech));
        }

        let total: i64 = count_query.select(count_star()).first(&mut conn).await?;

        let limit = params.limit.unwrap_or(50).clamp(1, 100);
        let offset = params.offset.unwrap_or(0).max(0);

        let records = query
            .order(alerts::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load::<AlertEntity>(&mut conn)
            .await?;

        Ok((records, total))
    }

    async fn find_by_id(&self, target_id: Uuid) -> Result<Option<AlertEntity>, AppError> {
        let mut conn = self.pool.get().await?;

        let alert = alerts::table
            .filter(alerts::alert_id.eq(target_id))
            .first::<AlertEntity>(&mut conn)
            .await
            .optional()?;

        Ok(alert)
    }

    async fn update_status(
        &self,
        target_id: Uuid,
        req: UpdateAlertStatusRequest,
    ) -> Result<UpdateAlertStatusResponse, AppError> {
        let mut conn = self.pool.get().await?;

        let updated = diesel::update(alerts::table.filter(alerts::alert_id.eq(target_id)))
            .set(alerts::status.eq(&req.status))
            .get_result::<AlertEntity>(&mut conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => AppError::NotFound(format!("Alert with id '{target_id}' not found")),
                other => AppError::DatabaseError(other.to_string()),
            })?;

        Ok(UpdateAlertStatusResponse {
            alert_id: updated.alert_id,
            status: updated.status,
            updated_at: Utc::now(),
        })
    }
}
