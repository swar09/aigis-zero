#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use agent_core::config::AgentConfig;
pub use agent_core::orchestrator::get_os_version;
use edr_sdk::proto::fleet::RegisterRequest;
use edr_sdk::models::event::EventBatch;
use edr_sdk::models::heartbeat::HeartbeatRequest;
use fleet_client::FleetClient;

use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(name = "aigis-zero", version, about = "Aigis-Zero Agent")]
struct Args {
    /// Config path
    #[arg(short, long, default_value = "/etc/aigis-zero/config.toml")]
    config: PathBuf,

    /// Validate config and exit
    #[arg(long)]
    check: bool,

    /// Force re-enrollment
    #[arg(long)]
    enroll: bool,
}

fn save_node_id_to_config(path: &Path, node_id: Uuid) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    let mut in_agent = false;
    let mut inserted = false;
    for i in 0..lines.len() {
        if lines[i].trim() == "[agent]" {
            in_agent = true;
            continue;
        }
        if in_agent && lines[i].starts_with("node_id") {
            lines[i] = format!("node_id = \"{}\"", node_id);
            inserted = true;
            break;
        }
        if in_agent && lines[i].starts_with('[') {
            lines.insert(i, format!("node_id = \"{}\"", node_id));
            inserted = true;
            break;
        }
    }
    if in_agent && !inserted {
        lines.push(format!("node_id = \"{}\"", node_id));
    }
    std::fs::write(path, lines.join("\n"))?;
    Ok(())
}

fn parse_endpoint(endpoint: &str) -> (std::net::IpAddr, u16) {
    let clean = endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let host_port = clean.split('/').next().unwrap_or(clean);
    let parts: Vec<&str> = host_port.split(':').collect();
    let ip_str = parts.first().copied().unwrap_or("127.0.0.1");
    let ip_str = ip_str.trim_start_matches('[').trim_end_matches(']');
    let ip = ip_str
        .parse()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(50051);
    (ip, port)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Parse CLI
    let args = Args::parse();

    // 2. Root check
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Error: aigis-zero must be run as root");
        std::process::exit(1);
    }

    // Parse config
    let config_str = std::fs::read_to_string(&args.config).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read config file at {}: {}",
            args.config.display(),
            e
        )
    })?;
    let mut config: AgentConfig = toml::from_str(&config_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?;

    if args.check {
        println!("Config syntax is valid.");
        std::process::exit(0);
    }

    let format = match config.agent.log_format.as_str() {
        "json" => agent_tracing::LogFormat::Json,
        _ => agent_tracing::LogFormat::Human,
    };
    agent_tracing::init(&config.agent.log_level, format)?;
    info!("Starting Aigis-Zero Agent");

    // Install panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Agent panicked: {}", panic_info);
        let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Status("Agent panicked")]);
    }));

    // Create OsqueryClient and connect
    let collector = osquery_client::OsqueryCollector::new(osquery_client::OsqueryConfig {
        socket_path: config.osquery.socket_path.clone(),
        db_path: config.agent.event_buffer_db.clone(),
    })
    .await?;

    // Create EventBuffer
    let buffer = event_buffer::EventBuffer::new(
        &config.agent.event_buffer_db,
        config.agent.event_buffer_max,
    )?;
    let buffer = Arc::new(buffer);

    // Connect to fleet server
    let mut fleet = FleetClient::new(config.fleet.endpoint.clone());
    fleet
        .connect_with_retry(
            config.fleet.max_reconnect_attempts,
            Duration::from_secs(config.fleet.reconnect_interval_secs),
            None,
        )
        .await?;

    // Enrollment
    let node_id = if let (Some(node_id), false) = (config.agent.node_id, args.enroll) {
        node_id
    } else {
        let enrollment = fleet
            .enroll(RegisterRequest {
                hostname: hostname::get()?.to_string_lossy().to_string(),
                os_version: get_os_version(),
                agent_version: env!("CARGO_PKG_VERSION").to_string(),
                machine_id: std::fs::read_to_string("/etc/machine-id").unwrap_or_else(|_| "unknown".to_string()).trim().to_string(),
            })
            .await?;

        let parsed_node_id = Uuid::parse_str(&enrollment.node_id).unwrap_or_default();
        // Save node_id to config file
        save_node_id_to_config(&args.config, parsed_node_id)?;
        config.agent.node_id = Some(parsed_node_id);

        parsed_node_id
    };

    info!(%node_id, "Successfully enrolled/loaded node ID");

    let fleet = Arc::new(Mutex::new(fleet));

    // Start AgentCore (osquery loop + command listener)
    let agent_uuid = node_id.to_string();
    let _results_rx = collector.start(&agent_uuid).await;
    let osquery_collector = Arc::new(collector);
    let (fleet_ip, fleet_port) = parse_endpoint(&config.fleet.endpoint);

    let command_handler = agent_core::command_handler::CommandHandler {
        osquery: osquery_collector.clone(),
        isolation: isolation::IsolationManager::new(fleet_ip, fleet_port),
    };

    let agent_core = agent_core::AgentCore {
        shutdown: tokio_util::sync::CancellationToken::new(),
        osquery: osquery_collector,
        buffer: buffer.clone(),
        command_handler: Arc::new(command_handler),
        fleet_client: fleet.clone(),
    };

    tokio::spawn(async move {
        let _ = agent_core.run(&agent_uuid).await;
    });

    // Start heartbeat loop (every heartbeat_interval_secs)
    let fleet_hb = fleet.clone();
    let hb_interval = config.fleet.heartbeat_interval_secs;
    let hb_buffer = buffer.clone();
    let hb_node_id = node_id.to_string();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(hb_interval));
        loop {
            ticker.tick().await;
            
            let count = hb_buffer.len().await.unwrap_or(0) as i64;
            let req = HeartbeatRequest {
                node_id: hb_node_id.clone(),
                status: "healthy".to_string(),
                events_buffered: count,
            };

            let mut f = fleet_hb.lock().await;
            if let Err(e) = f.heartbeat(&req).await {
                warn!("Heartbeat failed: {}", e);
            }
        }
    });

    // Start event drain loop (every event_drain_interval_secs)
    let fleet_drain = fleet.clone();
    let drain_interval = config.agent.event_drain_interval_secs;
    let batch_size = config.agent.event_drain_batch;
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(drain_interval));
        loop {
            ticker.tick().await;
            if let Ok(events) = buffer.drain(batch_size as usize).await
                && !events.is_empty()
            {
                let parsed_events = events
                    .iter()
                    .filter_map(|e| serde_json::from_str::<serde_json::Value>(e).ok())
                    .collect::<Vec<_>>();

                if !parsed_events.is_empty() {
                    let batch = EventBatch {
                        node_id,
                        events: parsed_events,
                    };
                    let mut f = fleet_drain.lock().await;
                    match f.send_events(&batch).await {
                        Ok(ack) if ack.success => {
                            // Successfully sent
                        }
                        Ok(ack) => {
                            warn!(error = ?ack.error, "Fleet rejected event batch, re-queuing");
                            for event in events {
                                let _ = buffer.push(event).await;
                            }
                        }
                        Err(e) => {
                            warn!(?e, "Failed to send events to fleet, re-queuing");
                            for event in events {
                                let _ = buffer.push(event).await;
                            }
                        }
                    }
                }
            }
        }
    });

    // Watchdog task
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
        }
    });

    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Ready]);

    // Wait for shutdown signal
    let _ = tokio::signal::ctrl_c().await;
    info!("Ctrl-C received, shutting down");

    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Stopping]);

    Ok(())
}
