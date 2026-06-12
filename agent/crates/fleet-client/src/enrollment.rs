use crate::types::{EnrollmentResult, RegisterRequest};
use anyhow::Result;
use tonic::transport::Channel;

pub struct AgentEnrollment;

impl AgentEnrollment {
    pub async fn enroll(_channel: Channel, request: RegisterRequest) -> Result<EnrollmentResult> {
        tracing::info!("Enrolling agent: {:?}", request.hostname);

        // Return error for now so we fall back to degraded/offline mode.
        anyhow::bail!("Enrollment not yet implemented (Planned for Sprint 4)");
    }
}
