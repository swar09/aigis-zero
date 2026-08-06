pub mod codec;
pub mod types;

use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tracing::{error, info, warn};
use uuid::Uuid;

use edr_sdk::models::event::{EventAck, EventBatch};
use edr_sdk::models::heartbeat::{HeartbeatRequest, HeartbeatResponse};
use edr_sdk::proto::fleet::{
    AgentEvent, HeartbeatRequest as ProtoHeartbeatRequest, RegisterRequest, RegisterResponse,
    ServerCommand, fleet_service_client::FleetServiceClient,
};

pub struct FleetClient {
    endpoint: String,
    client: Option<FleetServiceClient<Channel>>,
    outbound_tx: Option<mpsc::Sender<AgentEvent>>,
    inbound_rx: Option<mpsc::Receiver<ServerCommand>>,
    node_id: Option<Uuid>,
    token: Option<String>,
}

impl FleetClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: None,
            outbound_tx: None,
            inbound_rx: None,
            node_id: None,
            token: None,
        }
    }

    pub async fn connect(&mut self, token: Option<&str>) -> Result<(), anyhow::Error> {
        info!(endpoint = %self.endpoint, "Connecting to fleet server");

        let channel = Channel::from_shared(self.endpoint.clone())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .connect()
            .await?;

        let mut client = FleetServiceClient::new(channel);
        self.client = Some(client.clone());
        self.token = token.map(|s| s.to_string());

        let (outbound_tx, outbound_rx) = mpsc::channel::<AgentEvent>(100);
        let (inbound_tx, inbound_rx) = mpsc::channel::<ServerCommand>(100);

        if let Some(t) = token {
            let stream = ReceiverStream::new(outbound_rx);
            let mut req = Request::new(stream);

            req.metadata_mut().insert(
                "authorization",
                MetadataValue::try_from(format!("Bearer {}", t))?,
            );

            let response = client.event_stream(req).await?;

            let mut inbound_stream = response.into_inner();

            tokio::spawn(async move {
                loop {
                    match inbound_stream.message().await {
                        Ok(Some(msg)) => {
                            if inbound_tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            // Graceful server-side close
                            info!("Fleet server closed the inbound command stream");
                            break;
                        }
                        Err(e) => {
                            error!(error = %e, "Error reading from fleet inbound stream");
                            break;
                        }
                    }
                }
            });
        }

        self.outbound_tx = Some(outbound_tx);
        self.inbound_rx = Some(inbound_rx);

        info!("Connected to fleet server");
        Ok(())
    }

    pub async fn connect_with_retry(
        &mut self,
        max_attempts: u32,
        base_delay: Duration,
        token: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.connect(token).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if max_attempts > 0 && attempt >= max_attempts {
                        return Err(e);
                    }
                    let delay = base_delay * 2u32.pow(attempt.min(5));
                    warn!(attempt, delay_ms = delay.as_millis(), error = %e, "Connection failed, retrying");
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    pub async fn enroll(
        &mut self,
        request: RegisterRequest,
    ) -> Result<RegisterResponse, anyhow::Error> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let response = client
            .register_agent(Request::new(request))
            .await?
            .into_inner();

        let node_uuid = Uuid::parse_str(&response.node_id).map_err(|e| {
            anyhow::anyhow!(
                "Fleet server returned malformed node_id '{}': {}",
                response.node_id,
                e
            )
        })?;
        self.node_id = Some(node_uuid);
        self.token = Some(response.token.clone());

        Ok(response)
    }

    pub async fn send_events(&mut self, batch: &EventBatch) -> Result<EventAck, anyhow::Error> {
        let tx = self
            .outbound_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Stream not connected"))?;

        for val in &batch.events {
            let node_id = val["node_id"].as_str().unwrap_or_default().to_string();
            let event_type = if let Some(s) = val["event_type"].as_str() {
                s.to_string()
            } else if let Some(i) = val["event_type"].as_i64() {
                match i {
                    0 => "osquery".to_string(),
                    1 => "process".to_string(),
                    2 => "file".to_string(),
                    3 => "network".to_string(),
                    _ => i.to_string(),
                }
            } else {
                "".to_string()
            };
            let payload = match serde_json::to_vec(&val["payload"]) {
                Ok(p) => p,
                Err(e) => {
                    error!(error = %e, "Failed to serialize event payload; skipping event");
                    continue;
                }
            };
            let timestamp_ns = val["timestamp_ns"].as_i64().unwrap_or_default();
            let sequence_id = val["sequence_id"].as_str().unwrap_or_default().to_string();

            let proto_event = AgentEvent {
                node_id,
                event_type,
                payload,
                timestamp_ns,
                sequence_id,
            };
            tx.send(proto_event)
                .await
                .map_err(|_| anyhow::anyhow!("Send channel closed"))?;
        }

        Ok(EventAck {
            success: true,
            error: None,
        })
    }

    pub async fn heartbeat(
        &mut self,
        request: &HeartbeatRequest,
    ) -> Result<HeartbeatResponse, anyhow::Error> {
        let req = ProtoHeartbeatRequest {
            node_id: self.node_id.map(|u| u.to_string()).unwrap_or_default(),
            status: request.status.clone(),
            events_buffered: request.events_buffered,
        };

        let client = self
            .client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let mut req_tonic = Request::new(req);

        if let Some(t) = &self.token {
            req_tonic.metadata_mut().insert(
                "authorization",
                MetadataValue::try_from(format!("Bearer {}", t))?,
            );
        }

        let response = client.heartbeat(req_tonic).await?.into_inner();

        Ok(HeartbeatResponse { ok: response.ok })
    }

    pub fn try_receive(&mut self) -> Result<Option<ServerCommand>, anyhow::Error> {
        let rx = self
            .inbound_rx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        match rx.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(anyhow::anyhow!(
                "Inbound channel closed (server disconnected)"
            )),
        }
    }

    pub async fn receive(&mut self) -> Result<Option<ServerCommand>, anyhow::Error> {
        let rx = self
            .inbound_rx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        Ok(rx.recv().await)
    }

    pub fn node_id(&self) -> Option<Uuid> {
        self.node_id
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_connection_establishment() {}
    #[tokio::test]
    async fn test_enrollment_request_response() {}
    #[tokio::test]
    async fn test_event_batch_sending() {}
    #[tokio::test]
    async fn test_heartbeat_sending() {}
    #[tokio::test]
    async fn test_reconnection_after_disconnect() {}
    #[tokio::test]
    async fn test_invalid_server_response() {}
}
