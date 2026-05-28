use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

pub mod proto {
    tonic::include_proto!("edr.fleet");
}

use proto::fleet_service_server::{FleetService, FleetServiceServer};
use proto::{
    AgentConfig, AgentEvent, HeartbeatRequest, HeartbeatResponse, OsquerySchedule,
    RegisterRequest, RegisterResponse, ServerCommand,
};

#[derive(Default)]
pub struct MockFleetService {}

#[tonic::async_trait]
impl FleetService for MockFleetService {
    async fn register_agent(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Agent registered: {:?}", req.hostname);

        let config = AgentConfig {
            osquery_schedule: vec![OsquerySchedule {
                name: "running_processes".to_string(),
                query: "SELECT pid, name, path, cmdline, uid, parent FROM processes;".to_string(),
                interval_secs: 30,
            }],
            heartbeat_interval_secs: 30,
            batch_size: 100,
        };

        let response = RegisterResponse {
            node_id: Uuid::new_v4().to_string(),
            token: "mock_jwt_token".to_string(),
            config: Some(config),
        };

        Ok(Response::new(response))
    }

    type EventStreamStream = ReceiverStream<Result<ServerCommand, Status>>;

    async fn event_stream(
        &self,
        request: Request<tonic::Streaming<AgentEvent>>,
    ) -> Result<Response<Self::EventStreamStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Ok(Some(event)) = in_stream.message().await {
                tracing::info!(
                    "Received event from node {}: type {}, payload size {} bytes",
                    event.node_id,
                    event.event_type,
                    event.payload.len()
                );
                
                // We could send an Ack command here
                let ack = ServerCommand {
                    command: Some(proto::server_command::Command::Ack(proto::AckCommand {
                        sequence_id: event.sequence_id,
                    })),
                };
                
                if let Err(e) = tx.send(Ok(ack)).await {
                    tracing::error!("Failed to send ACK to client: {}", e);
                    break;
                }
            }
            tracing::info!("Event stream closed by client");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        tracing::debug!(
            "Heartbeat from node {}: status={}, events_buffered={}",
            req.node_id,
            req.status,
            req.events_buffered
        );

        Ok(Response::new(HeartbeatResponse { ok: true }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let addr = "0.0.0.0:50051".parse()?;
    
    tracing::info!("Mock Fleet Server listening on {}", addr);

    let service = MockFleetService::default();

    Server::builder()
        .add_service(FleetServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}


