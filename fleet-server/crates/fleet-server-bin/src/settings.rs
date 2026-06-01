use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

/// Flat settings struct populated from `.env` + environment variables.
///
/// All fields map 1-to-1 to keys in `.env`.
/// Environment variables always win over the file.
#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_log_level")]
    pub rust_log: String,

    #[serde(default = "default_log_format")]
    pub log_format: String,

    // These fields are read by postgres-interface and kafka-handler once implemented.
    #[allow(dead_code)]
    pub database_url: Option<String>,
    #[allow(dead_code)]
    pub kafka_brokers: Option<String>,
    #[allow(dead_code)]
    pub kafka_topic_agents_events: Option<String>,

    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    50051
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "human".to_string()
}

fn default_jwt_secret() -> String {
    "change-me-in-production".to_string()
}

impl Settings {
    /// Loads settings by merging (in order of increasing priority):
    ///   1. Hardcoded defaults (via serde defaults above)
    ///   2. `.env` file if present in `env_path`
    ///   3. Actual environment variables
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if a present `.env` file cannot be parsed,
    /// or if a required field cannot be deserialised.
    pub fn load(env_path: &Path) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        if env_path.exists() {
            // The `config` crate can read .env-style files via its Ini source.
            // We use the Ini source rather than the `dotenv` crate to keep deps lean.
            builder = builder.add_source(File::from(env_path).format(config::FileFormat::Ini));
        }

        // Environment variables override file values. Prefix is empty so
        // PORT=50051 maps to `port`, RUST_LOG=debug maps to `rust_log`, etc.
        builder = builder.add_source(Environment::default().try_parsing(true));

        builder.build()?.try_deserialize()
    }
}
