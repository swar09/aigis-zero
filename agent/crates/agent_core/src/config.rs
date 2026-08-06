use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

/// Root agent configuration. Read from /etc/aigis-zero/config.toml
#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub agent: AgentSection,
    pub osquery: OsquerySection,
    pub fleet: FleetSection,
    pub isolation: IsolationSection,
}

/// Agent identity and runtime settings.
#[derive(Debug, Deserialize, Clone)]
pub struct AgentSection {
    /// UUID assigned after first enrollment. None = not yet enrolled.
    pub node_id: Option<Uuid>,
    pub name: String,
    pub log_level: String,
    pub log_format: String,
    pub log_dir: PathBuf,
    pub data_dir: PathBuf,
    pub event_buffer_db: PathBuf,
    pub event_buffer_max: u64,
    pub event_drain_batch: u32,
    pub event_drain_interval_secs: u64,
    pub server: AgentServerSection,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentServerSection {
    pub bind_addr: String,
    pub port: u16,
}

/// OSQuery daemon configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct OsquerySection {
    pub socket_path: PathBuf,
    pub conf_path: PathBuf,
    pub flags_path: PathBuf,
    pub pid_file: PathBuf,
    pub log_path: PathBuf,
    pub connect_timeout_secs: u64,
    pub query_timeout_secs: u64,
}

/// Fleet server connection settings.
#[derive(Debug, Deserialize, Clone)]
pub struct FleetSection {
    pub host: String,
    pub port: u16,
    pub endpoint: String,
    pub enrollment_secret: String,
    pub tls_ca_cert: PathBuf,
    pub tls_client_cert: Option<PathBuf>,
    pub tls_client_key: Option<PathBuf>,
    pub heartbeat_interval_secs: u64,
    pub reconnect_interval_secs: u64,
    pub max_reconnect_attempts: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IsolationSection {
    pub enabled: bool,
    pub fleet_ip: String,
    pub fleet_port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_toml() {
        let toml_str = r#"
[agent]
name = "test-agent"
log_level = "debug"
log_format = "human"
log_dir = "/var/log"
data_dir = "/var/lib"
event_buffer_db = "/var/lib/events.db"
event_buffer_max = 1000
event_drain_batch = 100
event_drain_interval_secs = 5

[agent.server]
bind_addr = "127.0.0.1"
port = 9100

[osquery]
socket_path = "/var/osquery/osquery.em"
conf_path = "/etc/osquery/osquery.conf"
flags_path = "/etc/osquery/osquery.flags"
pid_file = "/var/osquery/osqueryd.pidfile"
log_path = "/var/log/osquery"
connect_timeout_secs = 30
query_timeout_secs = 60

[fleet]
host = "fleet.test"
port = 8443
endpoint = "https://fleet.test:8443"
enrollment_secret = "secret"
tls_ca_cert = "/etc/ca.crt"
heartbeat_interval_secs = 60
reconnect_interval_secs = 10
max_reconnect_attempts = 0

[isolation]
enabled = false
fleet_ip = "192.168.1.100"
fleet_port = 8443
"#;

        let config: AgentConfig = toml::from_str(toml_str).expect("Failed to parse TOML");
        assert_eq!(config.agent.name, "test-agent");
        assert_eq!(config.agent.server.port, 9100);
        assert_eq!(
            config.osquery.socket_path.to_str().unwrap(),
            "/var/osquery/osquery.em"
        );
        assert_eq!(config.fleet.host, "fleet.test");
        assert!(!config.isolation.enabled);
    }
}
