use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub status: String,
    pub events_buffered: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeartbeatResponse {
    pub ok: bool,
}
