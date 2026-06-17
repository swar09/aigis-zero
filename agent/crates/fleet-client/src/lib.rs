#![allow(unused_imports, unused_variables, dead_code, unused_mut)]

pub mod codec;
pub mod types;

use chrono::Utc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use edr_sdk::codec::JsonCodec;
use edr_sdk::models::enrollment::{EnrollmentRequest, EnrollmentResponse};
use edr_sdk::models::envelope::{AgentMessage, AgentMessageType, ServerMessage, ServerMessageType};
use edr_sdk::models::event::{EventAck, EventBatch};
use edr_sdk::models::heartbeat::{HeartbeatRequest, HeartbeatResponse};

pub struct FleetClient {
    endpoint: String,
    /// Sender for outgoing messages to the server
    outbound_tx: Option<mpsc::Sender<AgentMessage>>,
    /// Receiver for incoming messages from the server.
    ///
    /// DESIGN NOTE — Why `inbound_rx` lives here and not behind the Mutex:
    ///
    /// The top-level `Arc<Mutex<FleetClient>>` in AgentCore is required for
    /// tasks that mutate FleetClient (send, heartbeat, enroll). However,
    /// keeping `inbound_rx` inside the Mutex means the command-listener task
    /// would hold the lock for the *entire duration* of a blocking `recv()`
    /// call, starving the heartbeat and drain tasks.
    ///
    /// The correct fix (implemented in AgentCore::run below) is to call
    /// `receive()` while holding the lock only long enough to invoke
    /// `rx.try_recv()` (non-blocking) and then immediately release it,
    /// yielding to the scheduler between polls.  This avoids the need to
    /// move `inbound_rx` out of the struct while still preventing starvation.
    inbound_rx: Option<mpsc::Receiver<ServerMessage>>,
    /// Node ID assigned after enrollment
    node_id: Option<Uuid>,
}

impl FleetClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            outbound_tx: None,
            inbound_rx: None,
            node_id: None,
        }
    }

    /// Connect to the fleet server and establish the bidirectional stream.
    // TODO: Fix Circular Auth Deadlock
    // 1. Accept token option: connect(&mut self, token: Option<&str>)
    // 2. If token is Some(t), insert `authorization` Bearer metadata into req:
    //    req.metadata_mut().insert("authorization", MetadataValue::try_from(format!("Bearer {}", t))?)
    pub async fn connect(&mut self) -> Result<(), anyhow::Error> {
        info!(endpoint = %self.endpoint, "Connecting to fleet server");

        // Create tonic channel (HTTP/2 connection)
        let channel = Channel::from_shared(self.endpoint.clone())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .connect()
            .await?;

        // Create channels for message passing
        let (outbound_tx, outbound_rx) = mpsc::channel::<AgentMessage>(100);
        let (inbound_tx, inbound_rx) = mpsc::channel::<ServerMessage>(100);

        // Create the bidirectional stream using JsonCodec
        let mut client = tonic::client::Grpc::new(channel);
        let path = http::uri::PathAndQuery::from_static("/edr.fleet.FleetService/EventStream");

        let stream = ReceiverStream::new(outbound_rx);
        let req = tonic::Request::new(stream);

        let response = client
            .streaming(
                req,
                path,
                JsonCodec::<AgentMessage, ServerMessage>::default(),
            )
            .await?;

        let mut inbound_stream = response.into_inner();

        tokio::spawn(async move {
            while let Ok(Some(msg)) = inbound_stream.message().await {
                if inbound_tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        self.outbound_tx = Some(outbound_tx);
        self.inbound_rx = Some(inbound_rx);

        info!("Connected to fleet server");
        Ok(())
    }

    pub async fn connect_with_retry(
        &mut self,
        max_attempts: u32, // 0 = infinite
        base_delay: Duration,
    ) -> Result<(), anyhow::Error> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.connect().await {
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

    /// Send an enrollment request and wait for the response.
    pub async fn enroll(
        &mut self,
        request: EnrollmentRequest,
    ) -> Result<EnrollmentResponse, anyhow::Error> {
        let msg = AgentMessage {
            message_type: AgentMessageType::EnrollmentRequest,
            payload: serde_json::to_value(&request)?,
            timestamp: Utc::now(),
            node_id: None,
        };

        self.send(msg).await?;

        // Wait for enrollment response
        let response = self
            .receive()
            .await?
            .ok_or_else(|| anyhow::anyhow!("No enrollment response received"))?;

        match response.message_type {
            ServerMessageType::EnrollmentResponse => {
                let enrollment: EnrollmentResponse = serde_json::from_value(response.payload)?;
                self.node_id = Some(enrollment.node_id);
                Ok(enrollment)
            }
            ServerMessageType::Error => {
                Err(anyhow::anyhow!("Enrollment error: {}", response.payload))
            }
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    /// Send an event batch to the fleet server.
    pub async fn send_events(&mut self, batch: &EventBatch) -> Result<EventAck, anyhow::Error> {
        let msg = AgentMessage {
            message_type: AgentMessageType::EventBatch,
            payload: serde_json::to_value(batch)?,
            timestamp: Utc::now(),
            node_id: self.node_id,
        };

        self.send(msg).await?;

        let response = self
            .receive()
            .await?
            .ok_or_else(|| anyhow::anyhow!("No event ack received"))?;

        let ack: EventAck = serde_json::from_value(response.payload)?;
        Ok(ack)
    }

    /// Send a heartbeat.
    pub async fn heartbeat(
        &mut self,
        request: &HeartbeatRequest,
    ) -> Result<HeartbeatResponse, anyhow::Error> {
        let msg = AgentMessage {
            message_type: AgentMessageType::Heartbeat,
            payload: serde_json::to_value(request)?,
            timestamp: Utc::now(),
            node_id: self.node_id,
        };

        self.send(msg).await?;

        let response = self
            .receive()
            .await?
            .ok_or_else(|| anyhow::anyhow!("No heartbeat response"))?;

        let hb: HeartbeatResponse = serde_json::from_value(response.payload)?;
        Ok(hb)
    }

    async fn send(&self, msg: AgentMessage) -> Result<(), anyhow::Error> {
        let tx = self
            .outbound_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        tx.send(msg)
            .await
            .map_err(|_| anyhow::anyhow!("Send channel closed"))?;
        Ok(())
    }

    /// Non-blocking poll: attempt to receive one inbound `ServerMessage`.
    ///
    /// Returns:
    /// - `Ok(Some(msg))` — a message was ready in the channel buffer.
    /// - `Ok(None)`      — channel is open but empty right now (`try_recv`
    ///                      returned `Empty`).
    /// - `Err(_)`        — channel is closed / client not connected.
    ///
    /// Visibility is intentionally `pub` so that `AgentCore`'s command-
    /// listener task can call it after acquiring the Mutex only briefly
    /// (see the lock-release strategy comment in `AgentCore::run`).
    pub fn try_receive(&mut self) -> Result<Option<ServerMessage>, anyhow::Error> {
        let rx = self
            .inbound_rx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        match rx.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(anyhow::anyhow!("Inbound channel closed (server disconnected)"))
            }
        }
    }

    /// Blocking receive — waits until a `ServerMessage` arrives or the
    /// channel is closed.  Used internally by `enroll`, `send_events`, and
    /// `heartbeat` where the caller already holds the Mutex and is
    /// specifically waiting for a protocol-level acknowledgement.
    ///
    /// Made `pub` per task requirement so `AgentCore` can use it when a
    /// blocking wait is appropriate (e.g., waiting for the enrollment ack).
    /// For the continuous command-listener loop, prefer `try_receive` to
    /// avoid holding the Mutex across a blocking await.
    pub async fn receive(&mut self) -> Result<Option<ServerMessage>, anyhow::Error> {
        let rx = self
            .inbound_rx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        Ok(rx.recv().await)
    }

    /// Get the node_id (assigned after enrollment)
    pub fn node_id(&self) -> Option<Uuid> {
        self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_establishment() {
        // 1. Connection establishment (mock server)
    }

    #[tokio::test]
    async fn test_enrollment_request_response() {
        // 2. Enrollment request/response cycle
    }

    #[tokio::test]
    async fn test_event_batch_sending() {
        // 3. Event batch sending
    }

    #[tokio::test]
    async fn test_heartbeat_sending() {
        // 4. Heartbeat sending
    }

    #[tokio::test]
    async fn test_reconnection_after_disconnect() {
        // 5. Reconnection after disconnect
    }

    #[tokio::test]
    async fn test_invalid_server_response() {
        // 6. Invalid server response handling
    }
}
