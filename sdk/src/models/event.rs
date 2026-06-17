use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventBatch {
    pub node_id: Uuid,
    pub events: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventAck {
    pub success: bool,
    pub error: Option<String>,
}
