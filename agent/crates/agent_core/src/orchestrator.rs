use crate::config::AgentConfig;
use anyhow::Result;
use event_buffer::EventBuffer;
use osquery_client::types::OsqueryResult;
use tokio::sync::mpsc;

pub async fn run() -> Result<()> {
    let config_path =
        std::env::var("EDR_AGENT_CONFIG").unwrap_or_else(|_| "agent.toml".to_string());

    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config file at {}: {}", config_path, e))?;

    let config: AgentConfig = toml::from_str(&config_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?;

    let format = match config.agent.log_format.as_str() {
        "json" => agent_tracing::LogFormat::Json,
        _ => agent_tracing::LogFormat::Human,
    };

    agent_tracing::init(&config.agent.log_level, format)?;
    tracing::info!("Starting Aigis-Zero Agent Orchestrator");

    let config_path_clone = config_path.clone();
    tokio::task::spawn_blocking(move || {
        use notify::{RecursiveMode, Watcher};
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create config watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(
            std::path::Path::new(&config_path_clone),
            RecursiveMode::NonRecursive,
        ) {
            tracing::warn!("Could not watch config file {}: {}", config_path_clone, e);
        } else {
            tracing::info!("Watching {} for changes", config_path_clone);
        }

        for res in rx {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        tracing::info!(
                            "Config file {} modified (Hot-reload to be implemented in future sprint)",
                            config_path_clone
                        );
                    }
                }
                Err(e) => tracing::error!("watch error: {:?}", e),
            }
        }
    });

    // EventBuffer wraps rusqlite::Connection which is !Send, so we keep it
    // on this task and never move it into tokio::spawn.
    let buffer = EventBuffer::new(&config.agent.event_buffer_db, config.agent.event_buffer_max)
        .map_err(|e| anyhow::anyhow!("Failed to open event buffer: {}", e))?;
    tracing::info!(
        "Initialized event buffer at {:?}",
        config.agent.event_buffer_db
    );

    // Start OsqueryCollector
    let collector = osquery_client::OsqueryCollector::new(osquery_client::OsqueryConfig {
        socket_path: config.osquery.socket_path.clone(),
        db_path: config.agent.event_buffer_db.clone(),
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
    let mut fleet_client = fleet_client::FleetClient::new(config.fleet.endpoint.clone());

    let req = edr_sdk::proto::fleet::RegisterRequest {
        hostname: hostname_or_default(),
        os_version: get_os_version(),
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
                match buffer.push(bytes).await {
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

fn hostname_or_default() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

pub fn read_machine_id() -> String {
    if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "unknown-machine-id".to_string()
}

pub fn get_os_version() -> String {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    let path = Path::new("/etc/os-release");
    if !path.exists() {
        return "Unknown Linux (os-release not found)".to_string();
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            return "Unknown Linux (unable to open os-release)".to_string();
        }
    };

    let reader = BufReader::new(file);
    let mut name = None;
    let mut version = None;
    let mut pretty_name = None;

    for line_content in reader.lines().map_while(Result::ok) {
        let trimmed = line_content.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim();
            let val = trimmed[pos + 1..].trim().trim_matches('"').to_string();

            match key {
                "PRETTY_NAME" => pretty_name = Some(val),
                "NAME" => name = Some(val),
                "VERSION" => version = Some(val),
                _ => {}
            }
        }
    }

    if let Some(pretty) = pretty_name {
        pretty
    } else {
        let os_name = name.unwrap_or_else(|| "Linux".to_string());
        if let Some(ver) = version {
            format!("{} {}", os_name, ver)
        } else {
            os_name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osquery_client::types::{ColumnEntry, OsqueryResultRow, ResultAction};
    use serde_json::Value;

    #[test]
    fn test_encode_result() {
        let result = OsqueryResult {
            query_name: "test_query".to_string(),
            agent_uuid: "uuid-123".to_string(),
            timestamp_ns: 123456789,
            rows: vec![OsqueryResultRow {
                columns: vec![ColumnEntry {
                    name: "col1".to_string(),
                    value: "val1".to_string(),
                }],
            }],
            action: ResultAction::Snapshot,
        };

        let encoded = encode_result(&result);

        let parsed: fleet_client::types::AgentEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(parsed.node_id, "uuid-123");
        assert_eq!(
            parsed.event_type,
            fleet_client::types::EventType::Osquery as i32
        );
        assert_eq!(parsed.timestamp_ns, 123456789);

        let payload: Value = parsed.payload;
        assert_eq!(payload["query_name"].as_str().unwrap(), "test_query");
        assert_eq!(payload["action"].as_str().unwrap(), "SNAPSHOT");
    }
}
