use serde::{Deserialize, Serialize};

/// Type of event being sent from agent to fleet server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Osquery = 0,
    Process = 1,
    File = 2,
    Network = 3,
}

/// Current operational status of the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Healthy = 0,
    Degraded = 1,
    Isolated = 2,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Healthy => "healthy",
            AgentStatus::Degraded => "degraded",
            AgentStatus::Isolated => "isolated",
        }
    }
}

/// Connection state of the gRPC channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Reconnecting,
    Disconnected,
}

/// Sent by the agent to register with the fleet server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub hostname: String,
    pub os_version: String,
    pub agent_version: String,
    /// Read from /etc/machine-id on Linux
    pub machine_id: String,
}

/// Returned by the fleet server after successful enrollment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    /// UUID assigned by the fleet server — the agent's permanent identity
    pub node_id: String,

    /// JWT token for authenticating subsequent gRPC calls
    pub token: String,

    /// Initial agent configuration (scheduled queries, intervals, etc.)
    pub config: Option<AgentConfigPayload>,
}

/// Result type returned after enrollment completes.
/// (Not a protobuf message — internal Rust type only)
#[derive(Debug, Clone)]
pub struct EnrollmentResult {
    pub node_id: String,
    pub token: String,
    pub config: Option<AgentConfigPayload>,
}

/// An event sent from the agent to the fleet server over the
/// bidirectional gRPC stream. The `payload` field contains
/// a structured JSON value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentEvent {
    /// UUID of the agent (assigned during enrollment)
    pub node_id: String,

    /// Type of event: 0=osquery, 1=process, 2=file, 3=network
    pub event_type: i32,

    /// Pure JSON payload for native nested JSON serialization
    pub payload: serde_json::Value,

    /// Timestamp in nanoseconds since Unix epoch
    pub timestamp_ns: i64,

    /// UUID v4 for deduplication and acknowledgment tracking
    pub sequence_id: String,
}

/// A command sent from the fleet server to the agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerCommand {
    pub command: Option<ServerCommandType>,
}

/// The actual command variant.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerCommandType {
    Isolate(IsolateCommand),
    ConfigUpdate(ConfigUpdateCommand),
    Ack(AckCommand),
}

/// Command to isolate or de-isolate the node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolateCommand {
    /// true = isolate (block all traffic except fleet server)
    /// false = de-isolate (restore normal networking)
    pub isolate: bool,

    /// Human-readable reason for the isolation
    pub reason: String,
}

/// Command to update the agent's configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigUpdateCommand {
    pub config: Option<AgentConfigPayload>,
}

/// Acknowledgment that the server received a specific event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AckCommand {
    /// The sequence_id of the AgentEvent being acknowledged
    pub sequence_id: String,
}

/// Configuration payload sent from fleet server to agent.
/// Stored locally in SQLite after receipt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfigPayload {
    /// List of scheduled queries to execute via OSQuery
    pub osquery_schedule: Vec<OsquerySchedule>,

    /// How often to send heartbeats (seconds)
    pub heartbeat_interval_secs: i32,

    /// Max number of events to batch before sending
    pub batch_size: i32,
}

/// A single scheduled query definition (from fleet server config).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsquerySchedule {
    /// Unique name (e.g., "running_processes")
    pub name: String,

    /// SQL query to execute
    pub query: String,

    /// Interval in seconds
    pub interval_secs: i32,
}

/// Periodic heartbeat sent from agent to fleet server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    /// Agent's UUID
    pub node_id: String,

    /// Current status: "healthy" | "degraded" | "isolated"
    pub status: String,

    /// Number of events currently buffered locally in SQLite
    pub events_buffered: i64,
}

/// Fleet server's response to a heartbeat.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
}
