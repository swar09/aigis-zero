use crate::types::ConnectionState;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::watch;
use tonic::transport::{Channel, Endpoint};

pub struct FleetConnection {
    channel: Option<Channel>,
    endpoint: String,
    state_tx: watch::Sender<ConnectionState>,
}

impl FleetConnection {
    #[must_use]
    pub fn new(endpoint: &str) -> (Self, watch::Receiver<ConnectionState>) {
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        (
            Self {
                channel: None,
                endpoint: endpoint.to_string(),
                state_tx,
            },
            state_rx,
        )
    }

    pub async fn connect(&mut self) -> Result<Channel> {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);

        loop {
            let _ = self.state_tx.send(ConnectionState::Reconnecting);
            tracing::info!("Connecting to fleet server at {}...", self.endpoint);

            match Endpoint::from_shared(self.endpoint.clone()) {
                Ok(endpoint) => match endpoint.connect().await {
                    Ok(channel) => {
                        tracing::info!("Successfully connected to fleet server.");
                        let _ = self.state_tx.send(ConnectionState::Connected);
                        self.channel = Some(channel.clone());
                        return Ok(channel);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to connect to fleet server: {}. Retrying in {:?}",
                            e,
                            backoff
                        );
                    }
                },
                Err(e) => {
                    tracing::error!("Invalid fleet server endpoint {}: {}", self.endpoint, e);
                    // If endpoint is invalid, backoff and retry might not help, but we shouldn't panic.
                }
            }

            tokio::time::sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, max_backoff);
        }
    }
}
