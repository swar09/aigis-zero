use crate::config::AgentConfig;
use anyhow::Result;
use event_buffer::EventBuffer;
use osquery_client::types::{OsqueryResult, ScheduledQuery};
use serde::Deserialize;
use tokio::sync::mpsc;

/// Top-level structure of scheduled_queries.toml.
#[derive(Debug, Deserialize)]
struct ScheduledQueriesFile {
    queries: Vec<ScheduledQueryEntry>,
}

/// One entry inside [[queries]] in the TOML file.
#[derive(Debug, Deserialize)]
struct ScheduledQueryEntry {
    name: String,
    query: String,
    interval_secs: u64,
    #[serde(default)]
    snapshot: bool,
}

impl From<ScheduledQueryEntry> for ScheduledQuery {
    fn from(e: ScheduledQueryEntry) -> Self {
        ScheduledQuery {
            name: e.name,
            query: e.query,
            interval_secs: e.interval_secs,
            snapshot: e.snapshot,
        }
    }
}

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

    // EventBuffer wraps rusqlite::Connection which is !Send, so we keep it
    // on this task and never move it into tokio::spawn.
    let buffer = EventBuffer::new(&config.agent.buffer_path)
        .map_err(|e| anyhow::anyhow!("Failed to open event buffer: {}", e))?;
    tracing::info!("Initialized event buffer at {:?}", config.agent.buffer_path);

    // ── Seed scheduled queries from TOML file (testing only) ──────────────
    if let Some(sq_path) = &config.agent.scheduled_queries_path {
        match seed_scheduled_queries(sq_path, &config) {
            Ok(n) => tracing::info!(
                "Seeded {} scheduled queries into SQLite from {:?}",
                n,
                sq_path
            ),
            Err(e) => tracing::warn!("Could not seed scheduled queries from {:?}: {}", sq_path, e),
        }
    }

    // Start OsqueryCollector
    let collector = osquery_client::OsqueryCollector::new(osquery_client::OsqueryConfig {
        socket_path: config.osquery.socket_path.clone(),
        db_path: config.agent.buffer_path.clone(),
    })
    .await?;

    // agent_uuid: use node_id from config if available, otherwise a placeholder.
    let agent_uuid = config
        .agent
        .node_id
        .map(|u| u.to_string())
        .unwrap_or_else(|| "unregistered".to_string());

    let mut results_rx = collector.start(&agent_uuid).await;
    tracing::info!("OsqueryCollector started (agent_uuid={})", agent_uuid);

    // Fleet enrollment (non-fatal, fleet server not ready yet)
    tracing::info!("Attempting fleet enrollment (non-fatal if server is down)...");
    let mut fleet_client = fleet_client::FleetClient::new(fleet_client::FleetConfig {
        endpoint: config.fleet.endpoint.clone(),
    })
    .await?;

    let req = fleet_client::types::RegisterRequest {
        hostname: hostname_or_default(),
        os_version: "linux".to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        machine_id: read_machine_id(),
    };

    match tokio::time::timeout(std::time::Duration::from_secs(5), fleet_client.enroll(req)).await {
        Ok(Ok(enrollment)) => {
            tracing::info!("Enrolled with fleet server. node_id={}", enrollment.node_id);
        }
        Ok(Err(e)) => {
            tracing::warn!("Fleet enrollment failed (will continue offline): {}", e);
        }
        Err(_) => {
            tracing::warn!("Fleet enrollment timed out after 5s — running in offline mode.");
        }
    }

    // Main loop — drain results & handle shutdown
    // rusqlite::Connection is !Send so we drive the buffer writes here on the
    // main task rather than in a spawned task.
    tracing::info!("Agent is running. Draining osquery results. Press Ctrl-C to stop.");

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Ctrl-C received, signalling shutdown.");
        let _ = shutdown_tx.send(()).await;
    });

    loop {
        tokio::select! {
            Some(result) = results_rx.recv() => {
                let bytes = encode_result(&result);
                match buffer.push(&bytes) {
                    Ok(()) => tracing::debug!(
                        "Buffered '{}' ({} rows, action={:?})",
                        result.query_name,
                        result.rows.len(),
                        result.action,
                    ),
                    Err(e) => tracing::error!("Failed to buffer result: {}", e),
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Shutting down agent.");
                break;
            }
        }
    }

    Ok(())
}

/// Encode an OsqueryResult into a JSON AgentEvent string for the event buffer.
fn encode_result(result: &OsqueryResult) -> String {
    let payload = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    let event = fleet_client::types::AgentEvent {
        node_id: result.agent_uuid.clone(),
        event_type: fleet_client::types::EventType::Osquery as i32,
        payload,
        timestamp_ns: result.timestamp_ns,
        sequence_id: uuid::Uuid::new_v4().to_string(),
    };
    serde_json::to_string(&event).unwrap_or_default()
}

/// Load scheduled_queries.toml, upsert into the scheduler's SQLite table.
/// Returns the number of queries upserted.
fn seed_scheduled_queries(toml_path: &std::path::Path, config: &AgentConfig) -> Result<usize> {
    let content = std::fs::read_to_string(toml_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {:?}: {}", toml_path, e))?;

    let file: ScheduledQueriesFile = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {:?}: {}", toml_path, e))?;

    let queries: Vec<ScheduledQuery> = file.queries.into_iter().map(Into::into).collect();
    let n = queries.len();

    let mut scheduler = osquery_client::scheduler::QueryScheduler::new(&config.agent.buffer_path)?;
    scheduler.upsert_queries(&queries)?;

    Ok(n)
}

fn hostname_or_default() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

fn read_machine_id() -> String {
    std::fs::read_to_string("/etc/machine-id")
        .unwrap_or_default()
        .trim()
        .to_string()
}
