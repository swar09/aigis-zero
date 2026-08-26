use uuid::Uuid;

#[derive(Clone)]
pub struct FleetClient {
    pub target_url: String,
}

impl FleetClient {
    pub fn new(target_url: String) -> Self {
        Self { target_url }
    }

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
