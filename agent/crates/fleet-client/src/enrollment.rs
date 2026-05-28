use crate::types::{EnrollmentResult, RegisterRequest, RegisterResponse};
use anyhow::Result;
use tonic::{Request, client::Grpc, codec::ProstCodec, transport::Channel};

pub struct AgentEnrollment;

impl AgentEnrollment {
    pub async fn enroll(channel: Channel, request: RegisterRequest) -> Result<EnrollmentResult> {
        tracing::info!("Enrolling agent: {:?}", request.hostname);

        let mut client = Grpc::new(channel);
        let path = http::uri::PathAndQuery::from_static("/edr.fleet.FleetService/RegisterAgent");

        let res = client
            .unary(Request::new(request), path, ProstCodec::default())
            .await?
            .into_inner();

        Ok(EnrollmentResult {
            node_id: res.node_id,
            token: res.token,
            config: res.config,
        })
    }
}
