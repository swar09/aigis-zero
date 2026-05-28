use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────
// Scheduled Query Definition (stored in local SQLite)
// ─────────────────────────────────────────────────────────

/// A single scheduled query. Pushed by fleet server, persisted in SQLite.
/// Fields are all primitive types for zero-cost SQLite row mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledQuery {
    /// Unique name for this query (e.g., "running_processes")
    pub name: String,
    /// SQL string to execute against OSQuery
    pub query: String,
    /// Execution interval in seconds
    pub interval_secs: u64,
    /// If true, return full table snapshot each time.
    /// If false, compute differential (added/removed rows).
    pub snapshot: bool,
}

// ─────────────────────────────────────────────────────────
// OSQuery Thrift Response Types
// ─────────────────────────────────────────────────────────

/// Raw response from an OSQuery Thrift query() call
#[derive(Debug, Clone)]
pub struct QueryResponse {
    pub status: QueryStatus,
    pub rows: Vec<OsqueryRow>,
}

/// Status returned by the OSQuery ExtensionManager
#[derive(Debug, Clone)]
pub struct QueryStatus {
    /// 0 = success, non-zero = error
    pub code: i32,
    /// Human-readable status message
    pub message: String,
}

/// A single row from an OSQuery query result.
/// Keys are column names, values are string representations.
pub type OsqueryRow = HashMap<String, String>;

// ─────────────────────────────────────────────────────────
// Processed Query Result (protobuf-encodable)
// ─────────────────────────────────────────────────────────

/// A complete, processed query result ready for downstream consumption.
/// Derives prost::Message for protobuf serialization — this is what
/// gets encoded into the AgentEvent.payload field.
#[derive(Clone, Message)]
pub struct OsqueryResult {
    /// Name of the scheduled query that produced this result
    #[prost(string, tag = "1")]
    pub query_name: String,

    /// UUID of the agent that produced this result
    #[prost(string, tag = "2")]
    pub agent_uuid: String,

    /// Unix timestamp in nanoseconds
    #[prost(int64, tag = "3")]
    pub timestamp_ns: i64,

    /// The result rows, each encoded as an OsqueryResultRow
    #[prost(message, repeated, tag = "4")]
    pub rows: Vec<OsqueryResultRow>,

    /// Whether this is a snapshot, added diff, or removed diff
    #[prost(enumeration = "ResultAction", tag = "5")]
    pub action: i32,
}

/// A single row in an OsqueryResult, represented as key-value pairs.
/// Protobuf doesn't have a native map-in-repeated, so we use a message
/// with repeated entries.
#[derive(Clone, Message)]
pub struct OsqueryResultRow {
    #[prost(message, repeated, tag = "1")]
    pub columns: Vec<ColumnEntry>,
}

/// A single column name-value pair within a row.
#[derive(Clone, Message)]
pub struct ColumnEntry {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// The type of result action for differential queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, prost::Enumeration)]
#[repr(i32)]
pub enum ResultAction {
    /// Full table dump (first execution or snapshot mode)
    Snapshot = 0,
    /// Differential: rows added since last execution
    Added = 1,
    /// Differential: rows removed since last execution
    Removed = 2,
}

// Implement prost::Enumeration for ResultAction so prost can encode it
impl ResultAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResultAction::Snapshot => "SNAPSHOT",
            ResultAction::Added => "ADDED",
            ResultAction::Removed => "REMOVED",
        }
    }
}
