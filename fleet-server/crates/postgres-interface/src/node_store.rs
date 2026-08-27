use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use node_enrollment::{
    error::NodeEnrollmentError,
    store::{NodeRecord, NodeStore},
};
use uuid::Uuid;

use crate::{
    models::{NewEnrollmentEventEntity, NewNodeEntity},
    pool::DbPool,
    schema::{enrollment_events, nodes},
};

/// PostgreSQL-backed implementation of `NodeStore` using `diesel-async`.
///
/// Thread-safe: `DbPool` is cheaply cloneable (`Arc` inside).
pub struct PgNodeStore {
    pool: DbPool,
}

impl PgNodeStore {
    /// Wraps an existing Diesel connection pool.
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NodeStore for PgNodeStore {
    /// Upserts a node by `machine_id` and writes an audit event atomically.
    ///
    /// Runs inside a transaction to prevent concurrent enrollment races.
    ///
    /// # Errors
    ///
    /// Returns `NodeEnrollmentError::Store` on any pool or query failure.
    async fn upsert_node(&self, record: NodeRecord) -> Result<String, NodeEnrollmentError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            tracing::error!(err = %e, "failed to get connection from pool");
            NodeEnrollmentError::Store(e.to_string())
        })?;

        let machine_id_log = record.machine_id.clone();

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                // Step 1: Check whether a node with this machine_id already exists.
                let existing: Option<Uuid> = nodes::table
                    .select(nodes::node_id)
                    .filter(nodes::machine_id.eq(&record.machine_id))
                    .first::<Uuid>(conn)
                    .await
                    .optional()?;

                let (node_id, event_type) = match existing {
                    None => {
                        let node_id = Uuid::new_v4();
                        let new_node = NewNodeEntity {
                            node_id,
                            machine_id: &record.machine_id,
                            hostname: &record.hostname,
                            os_version: &record.os_version,
                            agent_version: &record.agent_version,
                            agent_status: "healthy",
                            operator_status: "active",
                        };

                        diesel::insert_into(nodes::table)
                            .values(&new_node)
                            .execute(conn)
                            .await?;

                        (node_id, "new_enrollment")
                    }
                    Some(node_id) => {
                        diesel::update(nodes::table.filter(nodes::node_id.eq(node_id)))
                            .set((
                                nodes::hostname.eq(&record.hostname),
                                nodes::os_version.eq(&record.os_version),
                                nodes::agent_version.eq(&record.agent_version),
                                nodes::last_enrolled_at.eq(Utc::now()),
                            ))
                            .execute(conn)
                            .await?;

                        (node_id, "re_enrollment")
                    }
                };

                // Step 2: Write audit log event
                let new_event = NewEnrollmentEventEntity {
                    event_id: Uuid::new_v4(),
                    node_id,
                    event_type,
                    hostname: &record.hostname,
                    os_version: &record.os_version,
                    agent_version: &record.agent_version,
                    enrolled_at: Utc::now(),
                };

                diesel::insert_into(enrollment_events::table)
                    .values(&new_event)
                    .execute(conn)
                    .await?;

                Ok(node_id)
            })
        })
        .await
        .map_err(|e| {
            tracing::error!(
                err = %e,
                machine_id = %machine_id_log,
                "upsert node transaction failed"
            );
            NodeEnrollmentError::Store(e.to_string())
        })
        .map(|id| id.to_string())
    }
}
