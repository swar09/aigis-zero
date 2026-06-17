use crate::types::{EnrollmentResult, RegisterRequest};
use anyhow::Result;
use tonic::transport::Channel;

pub struct AgentEnrollment;

impl AgentEnrollment {
    pub async fn enroll(channel: Channel, request: RegisterRequest) -> Result<EnrollmentResult> {
        tracing::info!("Enrolling agent: {:?}", request.hostname);

        // TODO: Implement direct unary enrollment call
        // 1. Initialize `client = tonic::client::Grpc::new(channel)`
        // 2. Set path to "/edr.fleet.FleetService/RegisterAgent"
        // 3. Make unary call passing `request` and using `JsonCodec::<RegisterRequest, RegisterResponse>::default()`
        // 4. Return EnrollmentResult mapping fields from the server response (RegisterResponse)

        anyhow::bail!("Enrollment not yet implemented (Planned for Sprint 4)");
    }
}
