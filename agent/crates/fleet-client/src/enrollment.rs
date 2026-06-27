use crate::types::{EnrollmentResult, RegisterRequest, RegisterResponse};
use anyhow::Result;
use tonic::transport::Channel;
use edr_sdk::codec::JsonCodec;

pub struct AgentEnrollment;

impl AgentEnrollment {
    pub async fn enroll(channel: Channel, request: RegisterRequest) -> Result<EnrollmentResult> {
        tracing::info!("Enrolling agent: {:?}", request.hostname);

        let mut client = tonic::client::Grpc::new(channel);
        let path = http::uri::PathAndQuery::from_static("/edr.fleet.FleetService/RegisterAgent");
        let tonic_req = tonic::Request::new(request);
        
        let response = client
            .unary(
                tonic_req,
                path,
                JsonCodec::<RegisterRequest, RegisterResponse>::default()
            )
            .await?;

        let res = response.into_inner();

        Ok(EnrollmentResult {
            node_id: res.node_id,
            token: res.token,
            config: res.config,
        })
    }
}
