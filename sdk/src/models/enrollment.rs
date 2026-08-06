use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrollmentRequest {
    pub enrollment_secret: String,
    pub hostname: String,
    pub os_version: String,
    pub agent_version: String,
    pub platform: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrollmentResponse {
    pub node_id: Uuid,
    pub status: String,
}
