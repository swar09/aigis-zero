use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use health_tracker::{
    error::HealthTrackerError,
    store::{HealthStore, HeartbeatRecord},
};

/// PostgreSQL-backed implementation of `HealthStore`.
///
/// Thread-safe: `PgPool` is `Arc`-wrapped internally.
pub struct PgHealthStore {
    pool: PgPool,
}

impl PgHealthStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthStore for PgHealthStore {
    /// Appends a heartbeat row to `node_health` and updates `nodes.agent_status`.
    ///
    /// IMPORTANT: Only `agent_status` is updated on `nodes`. The `operator_status`
    /// column is NEVER touched here — it is exclusively written by operator commands.
    /// This enforces the security boundary: an agent cannot clear its isolation by
    /// sending a healthy heartbeat.
    ///
    /// Both writes are wrapped in a single transaction for consistency.
    ///
    /// # Errors
    ///
    /// Returns `HealthTrackerError::Store` if the `node_id` is not a valid UUID
    /// or if any database operation fails.
    async fn record_heartbeat(&self, record: HeartbeatRecord) -> Result<(), HealthTrackerError> {
        // Parse here so we surface the error before opening a transaction.
        let node_id: Uuid = record.node_id.parse().map_err(|e| {
            tracing::error!(err = %e, raw = %record.node_id, "invalid node_id uuid in heartbeat");
            HealthTrackerError::Store(format!("invalid node_id uuid: {e}"))
        })?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| HealthTrackerError::Store(e.to_string()))?;

        // Append time-series record.
        sqlx::query!(
            r#"
            INSERT INTO node_health (node_id, agent_status, events_buffered, recorded_at)
            VALUES ($1, $2, $3, $4)
            "#,
            node_id,
            record.agent_status,
            record.events_buffered,
            record.recorded_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(err = %e, node_id = %node_id, "node_health insert failed");
            HealthTrackerError::Store(e.to_string())
        })?;

        // Update the current-state snapshot on the node row.
        // ONLY agent_status — never operator_status.
        sqlx::query!(
            r#"
            UPDATE nodes
            SET agent_status = $1
            WHERE node_id = $2
            "#,
            record.agent_status,
            node_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(err = %e, node_id = %node_id, "nodes.agent_status update failed");
            HealthTrackerError::Store(e.to_string())
        })?;

        tx.commit()
            .await
            .map_err(|e| HealthTrackerError::Store(e.to_string()))?;

        Ok(())
    }
}
