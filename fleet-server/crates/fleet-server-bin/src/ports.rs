use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;

use fleet_manager::{
    AgentHeartbeat, AgentRegistration, EnrollmentPort, EventIngestPort, HeartbeatPort,
    IncomingEvent, OutgoingCommand, RegistrationResult,
};

// These are temporary stub implementations that will be replaced once
// node-enrollment, health-tracker, and kafka-handler are implemented.
// They compile, log what they receive, and return sensible responses.

pub struct StubEnrollment;

#[async_trait]
impl EnrollmentPort for StubEnrollment {
    async fn register_agent(
        &self,
        registration: AgentRegistration,
    ) -> Result<RegistrationResult, Status> {
        let node_id = uuid::Uuid::new_v4().to_string();

        // Build a JWT that expires in 24 hours.
        let token = build_stub_token(&node_id);

        tracing::info!(
            hostname    = %registration.hostname,
            machine_id  = %registration.machine_id,
            os_version  = %registration.os_version,
            node_id     = %node_id,
            "stub: agent enrolled"
        );

        Ok(RegistrationResult { node_id, token })
    }
}

pub struct StubHeartbeat;

#[async_trait]
impl HeartbeatPort for StubHeartbeat {
    async fn record_heartbeat(&self, heartbeat: AgentHeartbeat) -> Result<(), Status> {
        tracing::debug!(
            node_id         = %heartbeat.node_id,
            status          = %heartbeat.status,
            events_buffered = heartbeat.events_buffered,
            "stub: heartbeat received"
        );
        Ok(())
    }
}

pub struct StubEventIngest;

#[async_trait]
impl EventIngestPort for StubEventIngest {
    async fn ingest_event(&self, event: IncomingEvent) -> Result<Option<OutgoingCommand>, Status> {
        tracing::debug!(
            node_id     = %event.node_id,
            event_type  = %event.event_type,
            sequence_id = %event.sequence_id,
            payload_len = event.payload.len(),
            "stub: event received"
        );

        // Ack every event so the agent can clear its buffer.
        Ok(Some(OutgoingCommand::Ack {
            sequence_id: event.sequence_id,
        }))
    }
}

/// Builds a stub JWT signed with the same secret the listener will validate.
/// In production this is replaced by the real enrollment crate.
fn build_stub_token(node_id: &str) -> String {
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Serialize, Deserialize)]
    struct Claims {
        node_id: String,
        exp: usize,
    }

    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize
        + 86400; // 24 hours

    // The secret here must match what is loaded into GrpcListenerConfig.
    // fleet-server-bin passes it through — stubs don't hardcode it separately.
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".into());

    encode(
        &Header::default(),
        &Claims {
            node_id: node_id.to_string(),
            exp,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap_or_else(|_| "stub-token-encode-failed".into())
}

/// Convenience: wraps the stubs in `Arc` and returns the three port objects.
pub fn stub_ports() -> (
    Arc<StubEnrollment>,
    Arc<StubHeartbeat>,
    Arc<StubEventIngest>,
) {
    (
        Arc::new(StubEnrollment),
        Arc::new(StubHeartbeat),
        Arc::new(StubEventIngest),
    )
}
