/// Configuration for the gRPC listener.
///
/// Populated by `fleet-server-bin` from the `.env` file via the `config` crate
/// and injected into `GrpcServer::new`. This crate never reads env vars directly.
#[derive(Debug, Clone)]
pub struct GrpcListenerConfig {
    /// Interface to bind on, e.g. `"0.0.0.0"`.
    pub host: String,

    /// Port to listen on, e.g. `50051`.
    pub port: u16,

    /// Secret used to validate incoming JWT bearer tokens.
    pub jwt_secret: String,
}

impl GrpcListenerConfig {
    /// Formats `host:port` into a bind address string.
    #[must_use]
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
