use config::{Config, ConfigError, Environment};
use serde::Deserialize;

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

    // Required — no default. The server refuses to start without a valid DB URL.
    // Set DATABASE_URL in .env or as an environment variable.
    pub database_url: String,

    // Kafka is stubbed — keep optional until kafka-handler is implemented.
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
    pub fn load() -> Result<Self, ConfigError> {
        let builder = Config::builder()
            // Environment variables (including those loaded from .env) override defaults.
            .add_source(Environment::default().try_parsing(true));

        builder.build()?.try_deserialize()
    }
}
