use thiserror::Error;

/// All errors originating from the node-enrollment crate.
#[derive(Debug, Error)]
pub enum NodeEnrollmentError {
    /// The underlying store rejected or failed the operation.
    /// Message is intentionally opaque — never forward raw DB errors to agents.
    #[error("store error: {0}")]
    Store(String),

    /// JWT signing failed.
    /// In practice only fires if the secret is empty — catch at startup.
    #[error("token signing failed: {0}")]
    TokenSign(#[from] jsonwebtoken::errors::Error),

    /// System clock is unusable (before UNIX epoch).
    #[error("system clock error: {0}")]
    Clock(String),
}
