use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use jsonwebtoken::DecodingKey;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};

use fleet_manager::{
    AgentHeartbeat, AgentRegistration, EnrollmentPort, EventIngestPort, HeartbeatPort,
    IncomingEvent, OutgoingCommand,
};

use crate::auth::validate_token;

// Include the code generated from fleet.proto by build.rs.
// Lints are suppressed on generated code we do not own.
#[allow(
    clippy::doc_markdown,
    clippy::default_trait_access,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::wildcard_imports
)]
pub(crate) mod proto {
    tonic::include_proto!("edr.fleet");
}

pub use proto::fleet_service_server::{FleetService, FleetServiceServer};

use proto::{
    AckCommand, AgentEvent, HeartbeatRequest, HeartbeatResponse, RegisterRequest, RegisterResponse,
    ServerCommand, server_command::Command,
};

type EventStream = Pin<Box<dyn Stream<Item = Result<ServerCommand, Status>> + Send + 'static>>;

/// The gRPC service implementation. Holds Arc refs to the domain port traits
/// so it is cheaply cloneable and the handlers are stateless.
pub struct FleetServiceImpl {
    enrollment: Arc<dyn EnrollmentPort>,
    heartbeat: Arc<dyn HeartbeatPort>,
    event_ingest: Arc<dyn EventIngestPort>,
    decoding_key: DecodingKey,
}

impl FleetServiceImpl {
    pub fn new(
        enrollment: Arc<dyn EnrollmentPort>,
        heartbeat: Arc<dyn HeartbeatPort>,
        event_ingest: Arc<dyn EventIngestPort>,
        jwt_secret: &str,
    ) -> Self {
        Self {
            enrollment,
            heartbeat,
            event_ingest,
            decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
        }
    }
}

#[async_trait]
impl FleetService for FleetServiceImpl {
    // RegisterAgent is intentionally unauthenticated — the agent calls this
    // once to get its node_id and JWT token.
    async fn register_agent(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            hostname = %req.hostname,
            machine_id = %req.machine_id,
            "agent enrollment request"
        );

        let result = self
            .enrollment
            .register_agent(AgentRegistration {
                hostname: req.hostname,
                os_version: req.os_version,
                agent_version: req.agent_version,
                machine_id: req.machine_id,
            })
            .await?;

        Ok(Response::new(RegisterResponse {
            node_id: result.node_id,
            token: result.token,
            config: None,
        }))
    }

    type EventStreamStream = EventStream;

    async fn event_stream(
        &self,
        request: Request<Streaming<AgentEvent>>,
    ) -> Result<Response<Self::EventStreamStream>, Status> {
        let claims = validate_token(request.metadata(), &self.decoding_key)?;
        let node_id = claims.node_id.clone();

        tracing::debug!(node_id = %node_id, "event stream opened");

        let mut inbound = request.into_inner();
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<Result<ServerCommand, Status>>(64);
        let event_ingest = Arc::clone(&self.event_ingest);

        tokio::spawn(async move {
            while let Some(result) = inbound.next().await {
                match result {
                    Ok(event) => {
                        let incoming = IncomingEvent {
                            node_id: event.node_id.clone(),
                            event_type: event.event_type,
                            payload: event.payload,
                            timestamp_ns: event.timestamp_ns,
                            sequence_id: event.sequence_id.clone(),
                        };

                        match event_ingest.ingest_event(incoming).await {
                            Ok(Some(OutgoingCommand::Ack { sequence_id })) => {
                                let cmd = ServerCommand {
                                    command: Some(Command::Ack(AckCommand { sequence_id })),
                                };
                                if cmd_tx.send(Ok(cmd)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(None) => {}
                            Err(status) => {
                                tracing::warn!(
                                    node_id = %event.node_id,
                                    err = %status,
                                    "event ingest error"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!(err = %e, node_id = %node_id, "stream error from agent");
                        break;
                    }
                }
            }
            tracing::debug!(node_id = %node_id, "event stream closed");
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(cmd_rx)) as EventStream
        ))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        validate_token(request.metadata(), &self.decoding_key)?;

        let req = request.into_inner();
        tracing::debug!(
            node_id = %req.node_id,
            status = %req.status,
            events_buffered = req.events_buffered,
            "heartbeat received"
        );

        self.heartbeat
            .record_heartbeat(AgentHeartbeat {
                node_id: req.node_id,
                status: req.status,
                events_buffered: req.events_buffered,
            })
            .await?;

        Ok(Response::new(HeartbeatResponse { ok: true }))
    }
}
