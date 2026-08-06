use async_trait::async_trait;

use crate::error::NodeEnrollmentError;

/// Data supplied by the caller to create or update a node record.
///
/// The store assigns the `node_id` (UUID) — it is NOT part of this struct.
/// The caller receives the assigned UUID as the `Ok` value from `upsert_node`.
#[derive(Debug, Clone)]
pub struct NodeRecord {
    pub hostname: String,
    pub os_version: String,
    pub agent_version: String,
    /// Content of `/etc/machine-id`. Stable across reboots. Natural key for upsert.
    pub machine_id: String,
}

/// Persistence abstraction for node enrollment.
///
/// Concrete implementation: `postgres_interface::PgNodeStore`.
/// This trait exists so `node-enrollment` compiles and tests without a live DB.
#[async_trait]
pub trait NodeStore: Send + Sync + 'static {
    /// Inserts a new node or updates an existing one by `machine_id`.
    ///
    /// Also appends an audit row to `enrollment_events` atomically.
    ///
    /// Returns the node's UUID string. Stable across calls for the same `machine_id`.
    ///
    /// # Errors
    ///
    /// Returns `NodeEnrollmentError::Store` on any persistence failure.
    async fn upsert_node(&self, record: NodeRecord) -> Result<String, NodeEnrollmentError>;
}
