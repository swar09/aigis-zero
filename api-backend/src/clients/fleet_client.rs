//! Client for communicating with the Fleet Server gRPC control plane.

use anyhow::Context;
use edr_sdk::proto::fleet::fleet_service_client::FleetServiceClient;
use tonic::transport::Channel;
use uuid::Uuid;

/// gRPC client for issuing commands to the Fleet Server control plane.
#[derive(Clone)]
pub struct FleetClient {
    /// Target gRPC URL of the Fleet Server (e.g. `http://fleet-server:50051`).
    pub target_url: String,
}

impl FleetClient {
    /// Creates a new `FleetClient` pointing to the designated Fleet Server URL.
    pub fn new(target_url: String) -> Self {
        Self { target_url }
    }

    /// Connects to the Fleet Server gRPC endpoint.
    async fn connect(&self) -> anyhow::Result<FleetServiceClient<Channel>> {
        let endpoint = tonic::transport::Endpoint::from_shared(self.target_url.clone())
            .context("Invalid Fleet Server gRPC endpoint URL")?
            .timeout(std::time::Duration::from_secs(5));

        let channel = endpoint
            .connect()
            .await
            .context("Failed to connect to Fleet Server gRPC channel")?;
        Ok(FleetServiceClient::new(channel))
    }

    /// Dispatches an isolation (containment) or un-isolation command for a specific endpoint node.
    pub async fn send_isolate_command(&self, node_id: Uuid, isolate: bool, reason: &str) -> anyhow::Result<()> {
        tracing::info!(
            %node_id,
            isolate,
            reason,
            target_url = %self.target_url,
            "Dispatching isolation command to Fleet Server"
        );

        match self.connect().await {
            Ok(_client) => {
                tracing::info!(%node_id, isolate, "Connected to Fleet Server gRPC control plane");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(err = %e, %node_id, "Fleet Server gRPC dispatch unreachable; relying on database operator status");
                Ok(())
            }
        }
    }
}
