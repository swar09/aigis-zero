use std::future::Future;

use tonic::transport::Server;
use tokio_util::sync::CancellationToken;

use crate::{
    config::GrpcListenerConfig,
    error::Error,
    service::{FleetServiceImpl, FleetServiceServer},
};

/// Owns the tonic server and its lifecycle.
pub struct GrpcServer {
    config: GrpcListenerConfig,
    service: FleetServiceImpl,
}

impl GrpcServer {
    #[must_use]
    pub fn new(config: GrpcListenerConfig, service: FleetServiceImpl) -> Self {
        Self { config, service }
    }

    /// Binds and serves until `shutdown` resolves.
    ///
    /// # Errors
    ///
    /// Returns `Error::AddrParse` if `config.bind_addr()` is not a valid socket address.
    /// Returns `Error::Transport` if the tonic server fails to bind or encounters a fatal error.
    pub async fn serve_until_shutdown(
        self,
        shutdown: impl Future<Output = ()>,
    ) -> Result<(), Error> {
        let addr: std::net::SocketAddr = self.config.bind_addr().parse()?;

        tracing::info!(addr = %addr, "gRPC listener starting");

        Server::builder()
            .add_service(FleetServiceServer::new(self.service))
            .serve_with_shutdown(addr, shutdown)
            .await?;

        tracing::info!("gRPC listener stopped");
        Ok(())
    }
}

/// Convenience: resolves when the given `CancellationToken` is cancelled.
pub async fn shutdown_signal(token: CancellationToken) {
    token.cancelled().await;
}
