use prost::Message;
use serde::{Deserialize, Serialize};

/// Type of event being sent from agent to fleet server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EventType {
    Osquery = 0,
    Process = 1,
    File = 2,
    Network = 3,
}

/// Current operational status of the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
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
/// Proto tag numbers match fleet.proto RegisterRequest.
#[derive(Clone, Message)]
pub struct RegisterRequest {
    #[prost(string, tag = "1")]
    pub hostname: String,

    #[prost(string, tag = "2")]
    pub os_version: String,

    #[prost(string, tag = "3")]
    pub agent_version: String,

    /// Read from /etc/machine-id on Linux
    #[prost(string, tag = "4")]
    pub machine_id: String,
}

/// Returned by the fleet server after successful enrollment.
#[derive(Clone, Message)]
pub struct RegisterResponse {
    /// UUID assigned by the fleet server — the agent's permanent identity
    #[prost(string, tag = "1")]
    pub node_id: String,

    /// JWT token for authenticating subsequent gRPC calls
    #[prost(string, tag = "2")]
    pub token: String,

    /// Initial agent configuration (scheduled queries, intervals, etc.)
    #[prost(message, optional, tag = "3")]
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
/// protobuf-encoded event data (e.g., OsqueryResult.encode_to_vec()).
#[derive(Clone, Message)]
pub struct AgentEvent {
    /// UUID of the agent (assigned during enrollment)
    #[prost(string, tag = "1")]
    pub node_id: String,

    /// Type of event: 0=osquery, 1=process, 2=file, 3=network
    #[prost(int32, tag = "2")]
    pub event_type: i32,

    /// Protobuf-encoded payload (e.g., OsqueryResult bytes)
    #[prost(bytes = "vec", tag = "3")]
    pub payload: Vec<u8>,

    /// Timestamp in nanoseconds since Unix epoch
    #[prost(int64, tag = "4")]
    pub timestamp_ns: i64,

    /// UUID v4 for deduplication and acknowledgment tracking
    #[prost(string, tag = "5")]
    pub sequence_id: String,
}

/// A command sent from the fleet server to the agent.
/// Uses prost oneof to match the proto3 `oneof command { ... }`.
#[derive(Clone, Message)]
pub struct ServerCommand {
    #[prost(oneof = "ServerCommandType", tags = "1, 2, 3")]
    pub command: Option<ServerCommandType>,
}

/// The actual command variant (maps to proto3 oneof).
#[derive(Clone, prost::Oneof)]
pub enum ServerCommandType {
    #[prost(message, tag = "1")]
    Isolate(IsolateCommand),

    #[prost(message, tag = "2")]
    ConfigUpdate(ConfigUpdateCommand),

    #[prost(message, tag = "3")]
    Ack(AckCommand),
}

/// Command to isolate or de-isolate the node.
#[derive(Clone, Message)]
pub struct IsolateCommand {
    /// true = isolate (block all traffic except fleet server)
    /// false = de-isolate (restore normal networking)
    #[prost(bool, tag = "1")]
    pub isolate: bool,

    /// Human-readable reason for the isolation
    #[prost(string, tag = "2")]
    pub reason: String,
}

/// Command to update the agent's configuration.
#[derive(Clone, Message)]
pub struct ConfigUpdateCommand {
    #[prost(message, optional, tag = "1")]
    pub config: Option<AgentConfigPayload>,
}

/// Acknowledgment that the server received a specific event.
#[derive(Clone, Message)]
pub struct AckCommand {
    /// The sequence_id of the AgentEvent being acknowledged
    #[prost(string, tag = "1")]
    pub sequence_id: String,
}

/// Configuration payload sent from fleet server to agent.
/// Stored locally in SQLite after receipt.
#[derive(Clone, Message, Serialize, Deserialize)]
pub struct AgentConfigPayload {
    /// List of scheduled queries to execute via OSQuery
    #[prost(message, repeated, tag = "1")]
    pub osquery_schedule: Vec<OsquerySchedule>,

    /// How often to send heartbeats (seconds)
    #[prost(int32, tag = "2")]
    pub heartbeat_interval_secs: i32,

    /// Max number of events to batch before sending
    #[prost(int32, tag = "3")]
    pub batch_size: i32,
}

/// A single scheduled query definition (from fleet server config).
#[derive(Clone, Message, Serialize, Deserialize)]
pub struct OsquerySchedule {
    /// Unique name (e.g., "running_processes")
    #[prost(string, tag = "1")]
    pub name: String,

    /// SQL query to execute
    #[prost(string, tag = "2")]
    pub query: String,

    /// Interval in seconds
    #[prost(int32, tag = "3")]
    pub interval_secs: i32,
}

/// Periodic heartbeat sent from agent to fleet server.
#[derive(Clone, Message)]
pub struct HeartbeatRequest {
    /// Agent's UUID
    #[prost(string, tag = "1")]
    pub node_id: String,

    /// Current status: "healthy" | "degraded" | "isolated"
    #[prost(string, tag = "2")]
    pub status: String,

    /// Number of events currently buffered locally in SQLite
    #[prost(int64, tag = "3")]
    pub events_buffered: i64,
}

/// Fleet server's response to a heartbeat.
#[derive(Clone, Message)]
pub struct HeartbeatResponse {
    #[prost(bool, tag = "1")]
    pub ok: bool,
}
