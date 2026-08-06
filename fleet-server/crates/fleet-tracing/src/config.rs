/// Output format for log lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Pretty-printed, colored, human-readable. Use in development.
    #[default]
    Human,

    /// Structured JSON. Use in production and log aggregation pipelines.
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "human" | "pretty" => Ok(Self::Human),
            other => Err(format!(
                "unknown log format '{other}': expected 'json' or 'human'"
            )),
        }
    }
}

/// Configuration passed into [`crate::init`].
///
/// Populated by `fleet-server-bin` from the `.env` / `config` crate.
/// `fleet-tracing` never reads environment variables directly.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Minimum log level directive, e.g. `"info"` or `"fleet_server=debug,info"`.
    /// If `RUST_LOG` is set at runtime it overrides this value.
    pub log_level: String,

    /// Output format: human-readable or JSON.
    pub format: LogFormat,

    /// Service name embedded in every structured log line.
    /// Useful when multiple services ship logs to the same aggregator.
    pub service_name: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            format: LogFormat::Human,
            service_name: "fleet-server".to_string(),
        }
    }
}
