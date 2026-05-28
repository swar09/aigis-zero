#[tokio::main]
async fn main() -> anyhow::Result<()> {
    agent_core::orchestrator::run().await
}
