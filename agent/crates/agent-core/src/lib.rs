#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
pub mod command_handler;
pub mod config;
pub mod orchestrator;

use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
// Assume these exist
use command_handler::CommandHandler;
use event_buffer::EventBuffer;
use fleet_client::FleetClient;
use osquery_client::OsqueryCollector;

pub struct AgentCore {
    pub shutdown: CancellationToken,
    pub osquery: Arc<OsqueryCollector>,
    pub buffer: Arc<EventBuffer>,
    pub command_handler: Arc<CommandHandler>,
    pub fleet_client: Arc<tokio::sync::Mutex<FleetClient>>,
}

impl AgentCore {
    pub async fn run(&self) -> Result<()> {
        let shutdown = self.shutdown.clone();

        // Task 1: Osquery polling loop
        // TODO: Implement OSQuery Polling and Buffering Loop
        // 1. Change AgentCore::run signature to: pub async fn run(&self, agent_uuid: &str) -> Result<()>
        // 2. Obtain receiver via results_rx = self.osquery.start(agent_uuid).await
        // 3. Receive results in loop, map to fleet_client::types::AgentEvent, serialize to JSON, and buffer1.push(bytes).await
        let osquery = self.osquery.clone();
        let buffer1 = self.buffer.clone();
        let osquery_task = tokio::spawn(async move {
            // ... existing osquery loop ...
        });

        // Task 2: Command listener
        // TODO: Implement Command Listener Loop
        // 1. Make fleet_client.receive() method public
        // 2. Lock fleet client, call receive().await, and pass ServerMessage to cmd_handler.handle(msg).await
        let cmd_handler = self.command_handler.clone();
        let fleet = self.fleet_client.clone();
        let command_task = tokio::spawn(async move {
            // Listen for incoming ServerMessages
            // Process commands
            // Send responses
        });

        // Wait for shutdown
        shutdown.cancelled().await;
        osquery_task.abort();
        command_task.abort();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_osquery_loop_produces_events() {
        // 1. Osquery loop produces events
    }

    #[tokio::test]
    async fn test_command_handling() {
        // 2. Command handling (run_query, isolate, unisolate)
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        // 3. Shutdown signal stops all tasks
    }

    #[tokio::test]
    async fn test_event_buffer_integration() {
        // 4. Event buffer integration
    }
}
