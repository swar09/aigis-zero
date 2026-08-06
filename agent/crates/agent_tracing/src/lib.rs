use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt};

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Pretty-printed, colored, human-readable (for development)
    #[default]
    Human,
    /// Structured JSON (for production / log aggregation)
    Json,
}

/// Initialize the agent's tracing/logging infrastructure.
pub fn init(log_level: &str, format: LogFormat) -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    match format {
        LogFormat::Human => {
            fmt()
                .with_env_filter(filter)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .init();
        }
        LogFormat::Json => {
            fmt()
                .json()
                .with_env_filter(filter)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .init();
        }
    }

    Ok(())
}
