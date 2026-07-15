use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum LiveEvent {
    #[serde(rename = "log")]
    Log {
        node_id: String,
        hostname: String,
        event_type: String,
        payload: serde_json::Value,
        timestamp_ns: i64,
    },
    #[serde(rename = "alert")]
    Alert {
        id: String,
        node_id: String,
        hostname: String,
        severity: String,
        mitre_technique_id: Option<String>,
        description: String,
        threat_score: f32,
        timestamp_ns: i64,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat {
        node_id: String,
        agent_status: String,
        events_buffered: i64,
        timestamp_ns: i64,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum WsClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe {
        topics: Option<Vec<String>>,
        node_id: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    #[serde(rename = "pong")]
    Pong { timestamp: i64 },
    #[serde(rename = "subscribed")]
    Subscribed {
        topics: Vec<String>,
        node_id: Option<String>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}
