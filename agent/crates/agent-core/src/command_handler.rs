#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use edr_sdk::proto::fleet::{ServerCommand, server_command::Command};
use isolation::IsolationManager;
use osquery_client::OsqueryCollector;
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

pub struct CommandHandler {
    pub osquery: Arc<OsqueryCollector>,
    pub isolation: IsolationManager,
}

impl CommandHandler {
    /// Processes a server command and returns a JSON status response.
    ///
    /// Handles different command types with specific actions:
    /// - `Isolate`: Isolates or de-isolates the process based on the flag.
    /// - `ConfigUpdate`: Acknowledges the configuration update.
    /// - `Ack`: Acknowledges the message.
    ///
    /// # Errors
    ///
    /// Returns an error if the command is missing from the message or if an isolation operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() {
    /// let handler = CommandHandler { /* ... */ };
    /// let cmd = ServerCommand { command: Some(Command::Ack(())) };
    /// let result = handler.handle(cmd).await;
    /// assert!(result.is_ok());
    /// # }
    /// ```
    pub async fn handle(&self, msg: ServerCommand) -> Result<Value, String> {
        let command = msg.command.ok_or("missing command")?;

        match command {
            Command::Isolate(iso) => {
                if iso.isolate {
                    self.isolation.isolate().await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({"status": "isolated"}))
                } else {
                    self.isolation
                        .de_isolate()
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({"status": "unisolated"}))
                }
            }
            Command::ConfigUpdate(_cfg) => Ok(serde_json::json!({"status": "config_updated"})),
            Command::Ack(_) => Ok(serde_json::json!({"status": "acked"})),
        }
    }
}
