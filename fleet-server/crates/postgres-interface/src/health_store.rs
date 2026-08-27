use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use health_tracker::{
    error::HealthTrackerError,
    store::{HealthStore, HeartbeatRecord},
};
use uuid::Uuid;

use crate::{
    models::NewNodeHealthEntity,
    pool::DbPool,
    schema::{node_health, nodes},
};

/// PostgreSQL-backed implementation of `HealthStore` using `diesel-async`.
///
/// Thread-safe: `DbPool` is cheaply cloneable (`Arc` inside).
pub struct PgHealthStore {
    pool: DbPool,
}

impl PgHealthStore {
    /// Wraps an existing Diesel connection pool.
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthStore for PgHealthStore {
    /// Appends a heartbeat row to `node_health` and updates `nodes.agent_status`.
    ///
    /// IMPORTANT: Only `agent_status` is updated on `nodes`. The `operator_status`
    /// column is NEVER touched here — it is exclusively written by operator commands.
    ///
    /// Both writes are wrapped in a single transaction for consistency.
    ///
    /// # Errors
    ///
    /// Returns `HealthTrackerError::Store` on any pool, uuid, or query failure.
    async fn record_heartbeat(&self, record: HeartbeatRecord) -> Result<(), HealthTrackerError> {
        let node_id: Uuid = record.node_id.parse().map_err(|e| {
            tracing::error!(err = %e, raw = %record.node_id, "invalid node_id uuid in heartbeat");
            HealthTrackerError::Store(format!("invalid node_id uuid: {e}"))
        })?;

        let mut conn = self.pool.get().await.map_err(|e| {
            tracing::error!(err = %e, "failed to get connection from pool");
            HealthTrackerError::Store(e.to_string())
        })?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                let new_health = NewNodeHealthEntity {
                    health_id: Uuid::new_v4(),
                    node_id,
                    agent_status: &record.agent_status,
                    events_buffered: record.events_buffered,
                    recorded_at: record.recorded_at,
                };

                diesel::insert_into(node_health::table)
                    .values(&new_health)
                    .execute(conn)
                    .await?;

                diesel::update(nodes::table.filter(nodes::node_id.eq(node_id)))
                    .set(nodes::agent_status.eq(&record.agent_status))
                    .execute(conn)
                    .await?;

                Ok(())
            })
        })
        .await
        .map_err(|e| {
            tracing::error!(
                err = %e,
                node_id = %node_id,
                "record_heartbeat transaction failed"
            );
            HealthTrackerError::Store(e.to_string())
        })
    }
}
