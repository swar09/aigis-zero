#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use edr_sdk::proto::fleet::{ServerCommand, server_command::Command};
use serde_json::Value;
use tracing::{info, warn};
use isolation::IsolationManager;
use osquery_client::OsqueryCollector;
use std::sync::Arc;

pub struct CommandHandler {
    pub osquery: Arc<OsqueryCollector>,
    pub isolation: IsolationManager,
}

impl CommandHandler {
    pub async fn handle(&self, msg: ServerCommand) -> Result<Value, String> {
        let command = msg.command.ok_or("missing command")?;

        match command {
            Command::Isolate(iso) => {
                if iso.isolate {
                    self.isolation.isolate().await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({"status": "isolated"}))
                } else {
                    self.isolation.de_isolate().await.map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({"status": "unisolated"}))
                }
            }
            Command::ConfigUpdate(_cfg) => {
                Ok(serde_json::json!({"status": "config_updated"}))
            }
            Command::Ack(_) => {
                Ok(serde_json::json!({"status": "acked"}))
            }
        }
    }
}
