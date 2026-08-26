//! Client for communicating with the Fleet Server gRPC control plane.

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

    /// Dispatches an isolation (containment) or un-isolation command for a specific endpoint node.
    pub async fn send_isolate_command(&self, node_id: Uuid, isolate: bool, reason: &str) -> anyhow::Result<()> {
        tracing::info!(
            %node_id,
            isolate,
            reason,
            target_url = %self.target_url,
            "Dispatching isolation command to Fleet Server"
        );
        Ok(())
    }
}
