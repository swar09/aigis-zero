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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub status: QueryStatus,
    pub rows: Vec<OsqueryRow>,
}

/// Status returned by the OSQuery ExtensionManager
#[derive(Debug, Clone, Serialize, Deserialize)]
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
// Processed Query Result (JSON-encodable)
// ─────────────────────────────────────────────────────────

/// A complete, processed query result ready for downstream consumption.
/// This gets embedded into AgentEvent.payload as structured JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsqueryResult {
    /// Name of the scheduled query that produced this result
    pub query_name: String,

    /// UUID of the agent that produced this result
    pub agent_uuid: String,

    /// Unix timestamp in nanoseconds
    pub timestamp_ns: i64,

    /// The result rows, each encoded as an OsqueryResultRow
    pub rows: Vec<OsqueryResultRow>,

    /// Whether this is a snapshot, added diff, or removed diff
    pub action: ResultAction,
}

/// A single row in an OsqueryResult, represented as key-value pairs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsqueryResultRow {
    pub columns: Vec<ColumnEntry>,
}

/// A single column name-value pair within a row.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColumnEntry {
    pub name: String,
    pub value: String,
}

/// The type of result action for differential queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResultAction {
    /// Full table dump (first execution or snapshot mode)
    Snapshot = 0,
    /// Differential: rows added since last execution
    Added = 1,
    /// Differential: rows removed since last execution
    Removed = 2,
}

impl ResultAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResultAction::Snapshot => "SNAPSHOT",
            ResultAction::Added => "ADDED",
            ResultAction::Removed => "REMOVED",
        }
    }
}
