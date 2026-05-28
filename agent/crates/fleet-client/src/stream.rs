use crate::types::{AgentEvent, ServerCommand};
use anyhow::Result;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, client::Grpc, metadata::MetadataValue, transport::Channel};
use tonic_prost::ProstCodec;

pub struct EventStreamManager;

impl EventStreamManager {
    pub async fn start(
        channel: Channel,
        token: String,
        events_rx: mpsc::Receiver<AgentEvent>,
    ) -> Result<mpsc::Receiver<ServerCommand>> {
        let mut client = Grpc::new(channel);
        let path = http::uri::PathAndQuery::from_static("/edr.fleet.FleetService/EventStream");

        let mut req = Request::new(ReceiverStream::new(events_rx));
        let meta_token = MetadataValue::try_from(format!("Bearer {}", token))?;
        req.metadata_mut().insert("authorization", meta_token);

        let response = client
            .streaming(
                req,
                path,
                ProstCodec::<AgentEvent, ServerCommand>::default(),
            )
            .await?;
        let mut stream = response.into_inner();

        let (server_cmd_tx, server_cmd_rx) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Ok(Some(cmd)) = stream.message().await {
                if server_cmd_tx.send(cmd).await.is_err() {
                    break;
                }
            }
        });

        Ok(server_cmd_rx)
    }
}
