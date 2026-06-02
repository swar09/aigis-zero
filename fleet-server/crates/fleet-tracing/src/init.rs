use thiserror::Error;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{LogFormat, TracingConfig};

#[derive(Debug, Error)]
pub enum InitError {
    #[error("tracing subscriber is already initialised")]
    AlreadyInitialised,

    #[error("invalid log level directive '{directive}': {source}")]
    InvalidDirective {
        directive: String,
        source: tracing_subscriber::filter::ParseError,
    },
}

/// Initialises the global tracing subscriber for the fleet server.
///
/// Call exactly once at process startup, before spawning any tasks.
///
/// The subscriber respects the `RUST_LOG` environment variable if set;
/// otherwise it falls back to `config.log_level`.
///
/// # Errors
///
/// Returns `InitError::AlreadyInitialised` if called more than once.
/// Returns `InitError::InvalidDirective` if `config.log_level` is not a valid
/// `tracing_subscriber` filter directive.
pub fn init(config: &TracingConfig) -> Result<(), InitError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info"))
    });

    let service_name = config.service_name.clone();

    match config.format {
        LogFormat::Human => {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                // Print the service name in the prefix so local multi-service
                // setups are easy to distinguish.
                .with_ansi(true);

            Registry::default()
                .with(filter)
                .with(fmt_layer)
                .try_init()
                .map_err(|_| InitError::AlreadyInitialised)?;
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                // Flatten event fields into the top-level JSON object so
                // log aggregators (Loki, Elasticsearch, etc.) can index them.
                .with_current_span(true)
                .with_span_list(true);

            Registry::default()
                .with(filter)
                .with(fmt_layer)
                .try_init()
                .map_err(|_| InitError::AlreadyInitialised)?;
        }
    }

    // Emit a startup banner so it's immediately obvious in logs which
    // format and level were selected.
    tracing::info!(
        service = %service_name,
        format  = ?config.format,
        level   = %config.log_level,
        "tracing initialised"
    );

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::config::{LogFormat, TracingConfig};

    // Tracing is global state — the subscriber can only be set once per
    // process. These tests verify config parsing, not subscriber init.

    #[test]
    fn log_format_parses_json() {
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!("JSON".parse::<LogFormat>().unwrap(), LogFormat::Json);
    }

    #[test]
    fn log_format_parses_human() {
        assert_eq!("human".parse::<LogFormat>().unwrap(), LogFormat::Human);
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Human);
    }

    #[test]
    fn log_format_rejects_unknown() {
        assert!("xml".parse::<LogFormat>().is_err());
    }

    #[test]
    fn tracing_config_default_is_sane() {
        let cfg = TracingConfig::default();
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.format, LogFormat::Human);
        assert_eq!(cfg.service_name, "fleet-server");
    }
}
