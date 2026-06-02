use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::HealthTrackerError;

/// A single heartbeat record as stamped by the server.
///
/// `recorded_at` is assigned by `HealthTracker` — not the agent, not the DB.
/// This guarantees correct time-series ordering even with drifted agent clocks.
#[derive(Debug, Clone)]
pub struct HeartbeatRecord {
    /// Node UUID string. Must be parseable as a UUID by the store.
    pub node_id: String,

    /// Agent-reported operational status. Values: `"healthy"` | `"degraded"`.
    /// The agent NEVER reports `"isolated"` — that is an operator concept.
    pub agent_status: String,

    /// Events buffered locally on the agent and not yet delivered.
    pub events_buffered: i64,

    /// Server-side timestamp of when this heartbeat was processed.
    pub recorded_at: DateTime<Utc>,
}

/// Persistence abstraction for heartbeat data.
///
/// Concrete implementation: `postgres_interface::PgHealthStore`.
#[async_trait]
pub trait HealthStore: Send + Sync + 'static {
    /// Appends a heartbeat row and updates the node's current `agent_status`.
    ///
    /// The concrete implementation wraps both writes in a single transaction.
    /// It MUST NOT modify `operator_status` — that column is operator-only.
    ///
    /// # Errors
    ///
    /// Returns `HealthTrackerError::Store` on any persistence failure.
    async fn record_heartbeat(&self, record: HeartbeatRecord) -> Result<(), HealthTrackerError>;
}
