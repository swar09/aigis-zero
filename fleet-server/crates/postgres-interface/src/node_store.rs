use async_trait::async_trait;
use sqlx::PgPool;

use node_enrollment::{
    error::NodeEnrollmentError,
    store::{NodeRecord, NodeStore},
};

/// PostgreSQL-backed implementation of `NodeStore`.
///
/// Thread-safe: `PgPool` is an `Arc`-wrapped pool internally. Clone freely.
pub struct PgNodeStore {
    pool: PgPool,
}

impl PgNodeStore {
    /// Wraps an existing connection pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NodeStore for PgNodeStore {
    /// Upserts a node by `machine_id` and writes an audit event atomically.
    ///
    /// Uses an explicit SELECT → INSERT/UPDATE pattern inside a transaction
    /// to unambiguously determine whether this is a new or repeat enrollment
    /// without relying on `xmax` system column behaviour (which is not stable
    /// under all MVCC scenarios and cannot be type-checked by sqlx at compile
    /// time).
    ///
    /// Transaction steps:
    /// 1. `SELECT node_id FROM nodes WHERE machine_id = $1 FOR UPDATE`
    ///    — locks the row if it exists, returns `None` if not.
    /// 2a. If `None` (new node): `INSERT INTO nodes ...` — Postgres assigns UUID.
    /// 2b. If `Some(id)` (re-enroll): `UPDATE nodes SET ... WHERE node_id = $1`.
    /// 3. `INSERT INTO enrollment_events ...` with the appropriate `event_type`.
    /// 4. `COMMIT`.
    ///
    /// Returns the `node_id` UUID string.
    ///
    /// # Errors
    ///
    /// Returns `NodeEnrollmentError::Store` on any database failure.
    async fn upsert_node(&self, record: NodeRecord) -> Result<String, NodeEnrollmentError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            tracing::error!(err = %e, "failed to begin transaction");
            NodeEnrollmentError::Store(e.to_string())
        })?;

        // Step 1: Check whether a node with this machine_id already exists.
        // FOR UPDATE locks the row so concurrent enrollments from the same
        // machine_id are serialised.
        let existing = sqlx::query!(
            r#"
            SELECT node_id
            FROM   nodes
            WHERE  machine_id = $1
            FOR UPDATE
            "#,
            record.machine_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(err = %e, machine_id = %record.machine_id, "lookup failed");
            NodeEnrollmentError::Store(e.to_string())
        })?;

        let (node_id, event_type) = match existing {
            None => {
                // Step 2a: New node — let Postgres assign the UUID.
                let row = sqlx::query!(
                    r#"
                    INSERT INTO nodes (machine_id, hostname, os_version, agent_version)
                    VALUES ($1, $2, $3, $4)
                    RETURNING node_id
                    "#,
                    record.machine_id,
                    record.hostname,
                    record.os_version,
                    record.agent_version,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(err = %e, machine_id = %record.machine_id, "insert failed");
                    NodeEnrollmentError::Store(e.to_string())
                })?;

                (row.node_id, "new_enrollment")
            }
            Some(row) => {
                let node_id = row.node_id;

                // Step 2b: Existing node — update mutable fields.
                sqlx::query!(
                    r#"
                    UPDATE nodes
                    SET hostname          = $1,
                        os_version        = $2,
                        agent_version     = $3,
                        last_enrolled_at  = now()
                    WHERE node_id = $4
                    "#,
                    record.hostname,
                    record.os_version,
                    record.agent_version,
                    node_id,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(err = %e, node_id = %node_id, "update failed");
                    NodeEnrollmentError::Store(e.to_string())
                })?;

                (node_id, "re_enrollment")
            }
        };

        // Step 3: Audit log — append-only, never modified.
        sqlx::query!(
            r#"
            INSERT INTO enrollment_events
                (node_id, event_type, hostname, os_version, agent_version)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            node_id,
            event_type,
            record.hostname,
            record.os_version,
            record.agent_version,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(err = %e, node_id = %node_id, "audit log insert failed");
            NodeEnrollmentError::Store(e.to_string())
        })?;

        // Step 4: Commit.
        tx.commit().await.map_err(|e| {
            tracing::error!(err = %e, node_id = %node_id, "commit failed");
            NodeEnrollmentError::Store(e.to_string())
        })?;

        tracing::info!(
            node_id    = %node_id,
            machine_id = %record.machine_id,
            event_type = %event_type,
            "node upserted"
        );

        Ok(node_id.to_string())
    }
}
