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
    // ── Connection ──────────────────────────────────────
    /// Path to osqueryd's extension manager Unix socket
    /// Default: /var/osquery/osquery.em
    pub socket_path: PathBuf,

    /// Connection timeout in seconds when connecting to the socket
    pub connect_timeout_secs: Option<u64>,

    // ── Daemon Options (mirrors osquery.conf "options") ─
    pub options: OsqueryOptions,

    // ── Initial Scheduled Queries ───────────────────────
    /// Bootstrap queries (overridden by fleet server push)
    pub schedule: Vec<ScheduledQueryConfig>,

    // ── File Integrity Monitoring ───────────────────────
    /// FIM paths: category_name → list of glob paths
    /// e.g., { "etc": ["/etc/%%", "/etc/ssh/%%"] }
    pub file_paths: Option<HashMap<String, Vec<String>>>,

    // ── Query Packs ─────────────────────────────────────
    /// Named packs: pack_name → path_to_pack_conf_file
    pub packs: Option<HashMap<String, String>>,
}

/// OSQuery daemon option flags.
/// Maps to the "options" section of osquery.conf.
/// All fields are Optional — only set values override osquery defaults.
#[derive(Debug, Deserialize)]
pub struct OsqueryOptions {
    // ── Core Daemon ─────────────────────────────────────
    /// How config is retrieved: "filesystem" | "tls"
    pub config_plugin: Option<String>,
    /// Where to send logs: "filesystem" | "syslog" | "tls"
    pub logger_plugin: Option<String>,
    /// Disable all logging if true
    pub disable_logging: Option<bool>,
    /// Disable event-based tables if true
    pub disable_events: Option<bool>,
    /// Disable kernel audit subsystem if true
    pub disable_audit: Option<bool>,

    // ── Audit Subsystem ─────────────────────────────────
    /// Enable process execution events via audit
    pub audit_allow_process_events: Option<bool>,
    /// Enable socket events via audit
    pub audit_allow_sockets: Option<bool>,
    /// Enable config change events via audit
    pub audit_allow_config: Option<bool>,
    /// Attempt to persist audit rules across osquery restarts
    pub audit_persist: Option<bool>,

    // ── Performance ─────────────────────────────────────
    /// Maximum number of events to buffer (default 50000)
    pub events_max: Option<u64>,
    /// Randomize query start times by this percentage (0-100)
    pub schedule_splay_percent: Option<u32>,
    /// Resource watchdog aggressiveness level
    pub watchdog_level: Option<u32>,
    /// Number of worker threads for query dispatch
    pub worker_threads: Option<u32>,

    // ── Identity ────────────────────────────────────────
    /// How to identify the host: "hostname" | "uuid" | "instance" | "specified"
    pub host_identifier: Option<String>,
    /// Custom identifier string when host_identifier = "specified"
    pub specified_identifier: Option<String>,

    // ── Database ────────────────────────────────────────
    /// Path to the RocksDB database (default /var/osquery/osquery.db)
    pub database_path: Option<PathBuf>,

    // ── Security ────────────────────────────────────────
    /// Comma-delimited list of tables to disable
    pub disable_tables: Option<String>,
    /// Comma-delimited list of tables to explicitly enable
    pub enable_tables: Option<String>,

    // ── Time ────────────────────────────────────────────
    /// Log timestamps in UTC if true
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
