use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid or expired jwt token")]
    Unauthenticated,

    #[error("missing authorization header")]
    MissingAuthHeader,

    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
}
