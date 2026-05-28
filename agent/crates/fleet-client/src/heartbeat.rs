use crate::types::{HeartbeatRequest, HeartbeatResponse};
use anyhow::Result;
use std::time::Duration;
use tokio::time;
use tonic::{transport::Channel, Request, client::Grpc, codec::ProstCodec, metadata::MetadataValue};

pub struct HeartbeatManager;

impl HeartbeatManager {
    pub async fn start(
        channel: Channel,
        token: String,
        node_id: String,
        interval_secs: u64,
    ) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                tracing::debug!("Sending heartbeat for node: {}", node_id);
                
                let req_payload = HeartbeatRequest {
                    node_id: node_id.clone(),
                    status: "healthy".to_string(),
                    events_buffered: 0,
                };
                
                let mut client = Grpc::new(channel.clone());
                let path = http::uri::PathAndQuery::from_static("/edr.fleet.FleetService/Heartbeat");
                let mut req = Request::new(req_payload);
                if let Ok(meta_token) = MetadataValue::try_from(format!("Bearer {}", token)) {
                    req.metadata_mut().insert("authorization", meta_token);
                }
                
                if let Err(e) = client.unary(req, path, ProstCodec::<HeartbeatRequest, HeartbeatResponse>::default()).await {
                    tracing::warn!("Failed to send heartbeat: {}", e);
                }
            }
        });
        
        Ok(())
    }
}
