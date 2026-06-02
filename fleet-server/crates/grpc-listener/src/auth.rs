use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use tonic::{Status, metadata::MetadataMap};

use crate::error::Error;

/// Claims encoded inside the JWT token issued at enrollment.
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeClaims {
    /// The node's UUID, assigned during `RegisterAgent`.
    pub node_id: String,

    /// Standard JWT expiry (unix timestamp).
    pub exp: usize,
}

/// Extracts and validates the `Authorization: Bearer <token>` header.
///
/// Returns the decoded `NodeClaims` on success.
///
/// # Errors
///
/// Returns `Status::unauthenticated` if the header is missing, malformed,
/// uses a non-Bearer scheme, or contains an invalid/expired JWT.
pub fn validate_token(
    metadata: &MetadataMap,
    decoding_key: &DecodingKey,
) -> Result<NodeClaims, Status> {
    let header = metadata
        .get("authorization")
        .ok_or(Error::MissingAuthHeader)
        .map_err(|_| Status::unauthenticated("missing authorization header"))?;

    let header_str = header
        .to_str()
        .map_err(|_| Status::unauthenticated("authorization header is not valid utf-8"))?;

    let token = header_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("authorization header must use Bearer scheme"))?;

    let token_data =
        decode::<NodeClaims>(token, decoding_key, &Validation::default()).map_err(|e| {
            tracing::debug!(err = %e, "jwt validation failed");
            Status::unauthenticated("invalid or expired token")
        })?;

    Ok(token_data.claims)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use tonic::metadata::MetadataValue;

    fn make_token(node_id: &str, secret: &str, exp_offset_secs: i64) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let exp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + exp_offset_secs) as usize;
        let claims = NodeClaims {
            node_id: node_id.to_string(),
            exp,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn accepts_valid_token() {
        let secret = "test-secret";
        let token = make_token("node-123", secret, 3600);
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut map = MetadataMap::new();
        map.insert(
            "authorization",
            MetadataValue::try_from(format!("Bearer {token}")).unwrap(),
        );

        let claims = validate_token(&map, &decoding_key).unwrap();
        assert_eq!(claims.node_id, "node-123");
    }

    #[test]
    fn rejects_missing_header() {
        let decoding_key = DecodingKey::from_secret(b"secret");
        let map = MetadataMap::new();
        let err = validate_token(&map, &decoding_key).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn rejects_expired_token() {
        let secret = "test-secret";
        let token = make_token("node-123", secret, -3600);
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut map = MetadataMap::new();
        map.insert(
            "authorization",
            MetadataValue::try_from(format!("Bearer {token}")).unwrap(),
        );

        let err = validate_token(&map, &decoding_key).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn rejects_wrong_secret() {
        let token = make_token("node-123", "correct-secret", 3600);
        let decoding_key = DecodingKey::from_secret(b"wrong-secret");

        let mut map = MetadataMap::new();
        map.insert(
            "authorization",
            MetadataValue::try_from(format!("Bearer {token}")).unwrap(),
        );

        let err = validate_token(&map, &decoding_key).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }
}
