use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Root agent configuration. Read from /etc/edr/agent.toml
#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub fleet: FleetConfig,
    pub agent: AgentSection,
    pub osquery: OsqueryConfig,
}

/// Fleet server connection settings.
#[derive(Debug, Deserialize)]
pub struct FleetConfig {
    /// gRPC endpoint, e.g., "http://fleet.internal:50051"
    pub endpoint: String,

    /// Heartbeat interval in seconds (default 30, overridden by fleet server)
    pub heartbeat_interval_secs: u64,

    /// Max events per gRPC batch send
    pub batch_size: u32,
}

/// Agent identity and runtime settings.
#[derive(Debug, Deserialize)]
pub struct AgentSection {
    /// UUID assigned after first enrollment. None = not yet enrolled.
    pub node_id: Option<Uuid>,

    /// Path to the SQLite database for event buffering + query storage
    pub buffer_path: PathBuf,

    /// Log level filter: "trace" | "debug" | "info" | "warn" | "error"
    pub log_level: String,

    /// Log output format: "human" (default, colored) | "json" (structured)
    pub log_format: Option<String>,

    /// Path to a TOML file containing scheduled queries to seed into SQLite.
    /// Testing only — remove path from config (or omit field) in production.
    pub scheduled_queries_path: Option<PathBuf>,
}

/// OSQuery daemon configuration.
#[derive(Debug, Deserialize)]
pub struct OsqueryConfig {
    /// Path to osqueryd's extension manager Unix socket
    /// Default: /var/osquery/osquery.em
    pub socket_path: PathBuf,

    /// Connection timeout in seconds when connecting to the socket
    pub connect_timeout_secs: Option<u64>,

    // Daemon Options (mirrors osquery.conf "options")
    pub options: OsqueryOptions,

    /// Bootstrap queries (overridden by fleet server push)
    pub schedule: Vec<ScheduledQueryConfig>,

    /// FIM paths: category_name → list of glob paths
    /// e.g., { "etc": ["/etc/%%", "/etc/ssh/%%"] }
    pub file_paths: Option<HashMap<String, Vec<String>>>,

    /// Named packs: pack_name → path_to_pack_conf_file
    pub packs: Option<HashMap<String, String>>,
}

/// OSQuery daemon option flags.
/// Maps to the "options" section of osquery.conf.
/// All fields are Optional — only set values override osquery defaults.
#[derive(Debug, Deserialize)]
pub struct OsqueryOptions {
    pub config_plugin: Option<String>,
    pub logger_plugin: Option<String>,
    pub disable_logging: Option<bool>,
    pub disable_events: Option<bool>,
    pub disable_audit: Option<bool>,
    pub audit_allow_process_events: Option<bool>,
    pub audit_allow_sockets: Option<bool>,
    pub audit_allow_config: Option<bool>,
    pub audit_persist: Option<bool>,
    pub events_max: Option<u64>,
    pub schedule_splay_percent: Option<u32>,
    pub watchdog_level: Option<u32>,
    pub worker_threads: Option<u32>,
    pub host_identifier: Option<String>,
    pub specified_identifier: Option<String>,
    pub database_path: Option<PathBuf>,
    pub disable_tables: Option<String>,
    pub enable_tables: Option<String>,
    pub utc: Option<bool>,
}

/// A scheduled query definition in the TOML config file.
#[derive(Debug, Deserialize)]
pub struct ScheduledQueryConfig {
    pub name: String,
    pub query: String,
    pub interval_secs: u64,
    /// true = full snapshot each time, false = differential (default)
    pub snapshot: Option<bool>,
    /// Track removed rows in differential mode (default true)
    pub removed: Option<bool>,
    /// Restrict to specific platform: "linux" | "darwin" | "windows"
    pub platform: Option<String>,
}
