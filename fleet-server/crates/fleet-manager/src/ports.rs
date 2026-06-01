use async_trait::async_trait;
use tonic::Status;

// These types mirror the generated proto structs. We re-declare them here as
// plain Rust structs so fleet-manager has no compile-time dep on the generated
// code in grpc-listener. grpc-listener converts at the boundary.

/// Minimal registration request forwarded from the gRPC boundary.
#[derive(Debug, Clone)]
pub struct AgentRegistration {
    pub hostname: String,
    pub os_version: String,
    pub agent_version: String,
    pub machine_id: String,
}

/// Result returned to the gRPC boundary after successful enrollment.
#[derive(Debug, Clone)]
pub struct RegistrationResult {
    pub node_id: String,
    pub token: String,
}

/// Heartbeat forwarded from the gRPC boundary.
#[derive(Debug, Clone)]
pub struct AgentHeartbeat {
    pub node_id: String,
    pub status: String,
    pub events_buffered: i64,
}

/// Raw event bytes forwarded from the gRPC boundary.
#[derive(Debug, Clone)]
pub struct IncomingEvent {
    pub node_id: String,
    pub event_type: String,
    pub payload: Vec<u8>,
    pub timestamp_ns: i64,
    pub sequence_id: String,
}

/// Optional command to send back to the agent over the bidi stream.
#[derive(Debug, Clone)]
pub enum OutgoingCommand {
    Ack { sequence_id: String },
}

/// Drives agent enrollment. Implemented by `node-enrollment`.
#[async_trait]
pub trait EnrollmentPort: Send + Sync + 'static {
    async fn register_agent(
        &self,
        registration: AgentRegistration,
    ) -> Result<RegistrationResult, Status>;
}

/// Records agent heartbeats. Implemented by `health-tracker`.
#[async_trait]
pub trait HeartbeatPort: Send + Sync + 'static {
    async fn record_heartbeat(&self, heartbeat: AgentHeartbeat) -> Result<(), Status>;
}

/// Ingests agent events and forwards them. Implemented by `kafka-handler`.
#[async_trait]
pub trait EventIngestPort: Send + Sync + 'static {
    async fn ingest_event(&self, event: IncomingEvent) -> Result<Option<OutgoingCommand>, Status>;
}
