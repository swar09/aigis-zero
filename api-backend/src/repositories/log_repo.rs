use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    db::{DbPool, schema::event_logs},
    error::AppError,
    models::log::{EventLogEntity, LogFilterParams},
};

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn search_logs(&self, params: LogFilterParams) -> Result<Vec<EventLogEntity>, AppError>;
    async fn find_by_id(&self, target_id: Uuid) -> Result<Option<EventLogEntity>, AppError>;
}

#[derive(Clone)]
pub struct DieselLogRepository {
    pool: DbPool,
}

impl DieselLogRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LogRepository for DieselLogRepository {
    async fn search_logs(&self, params: LogFilterParams) -> Result<Vec<EventLogEntity>, AppError> {
        let mut conn = self.pool.get().await?;

        let mut query = event_logs::table.into_boxed();

        if let Some(nid) = params.node_id {
            query = query.filter(event_logs::node_id.eq(nid));
        }
        if let Some(et) = params.event_type {
            query = query.filter(event_logs::event_type.eq(et));
        }
        if let Some(from_t) = params.from_timestamp {
            query = query.filter(event_logs::recorded_at.ge(from_t));
        }
        if let Some(to_t) = params.to_timestamp {
            query = query.filter(event_logs::recorded_at.le(to_t));
        }

        let limit = params.limit.unwrap_or(100).clamp(1, 500);
        let offset = params.offset.unwrap_or(0).max(0);

        let records = query
            .order(event_logs::recorded_at.desc())
            .limit(limit)
            .offset(offset)
            .load::<EventLogEntity>(&mut conn)
            .await?;

        Ok(records)
    }

    async fn find_by_id(&self, target_id: Uuid) -> Result<Option<EventLogEntity>, AppError> {
        let mut conn = self.pool.get().await?;

        let log = event_logs::table
            .filter(event_logs::event_id.eq(target_id))
            .first::<EventLogEntity>(&mut conn)
            .await
            .optional()?;

        Ok(log)
    }
}
