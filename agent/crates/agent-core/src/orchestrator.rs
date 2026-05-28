use crate::config::AgentConfig;
use anyhow::Result;
use event_buffer::EventBuffer;
use fleet_client::{
    FleetClient,
    types::{AgentEvent, EventType, RegisterRequest},
};
use osquery_client::OsqueryCollector;
use prost::Message;

pub async fn run() -> Result<()> {
    let config_path =
        std::env::var("EDR_AGENT_CONFIG").unwrap_or_else(|_| "agent.toml".to_string());

    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config file at {}: {}", config_path, e))?;

    let config: AgentConfig = toml::from_str(&config_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?;

    let format = match config.agent.log_format.as_deref() {
        Some("json") => agent_tracing::LogFormat::Json,
        _ => agent_tracing::LogFormat::Human,
    };

    agent_tracing::init(&config.agent.log_level, format)?;
    tracing::info!("Starting EDR Agent Orchestrator");

    let _buffer = EventBuffer::new(&config.agent.buffer_path)?;
    tracing::info!("Initialized event buffer at {:?}", config.agent.buffer_path);

    let mut fleet_client = fleet_client::FleetClient::new(fleet_client::FleetConfig {
        endpoint: config.fleet.endpoint.clone(),
    })
    .await?;

    let req = RegisterRequest {
        hostname: "mock-hostname".to_string(),
        os_version: "mock-os".to_string(),
        agent_version: "0.1.0".to_string(),
        machine_id: "mock-machine-id".to_string(),
    };

    match fleet_client.enroll(req).await {
        Ok(enrollment) => {
            tracing::info!("Enrolled successfully with node_id: {}", enrollment.node_id);
        }
        Err(e) => {
            tracing::error!("Failed to enroll: {}", e);
            // We would continue and buffer locally, but stub for now.
        }
    }

    // Stub: We would also create OsqueryCollector and route events here.

    // Keeping main alive
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    Ok(())
}
