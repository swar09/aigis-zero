use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentMessage {
    pub message_type: AgentMessageType,
    pub payload: Value,
    pub timestamp: DateTime<Utc>,
    pub node_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMessageType {
    EnrollmentRequest,
    EventBatch,
    Heartbeat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerMessage {
    pub message_type: ServerMessageType,
    pub payload: Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMessageType {
    EnrollmentResponse,
    HeartbeatResponse,
    EventAck,
    Command,
    Error,
}
