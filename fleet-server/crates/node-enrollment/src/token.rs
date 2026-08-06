use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::NodeEnrollmentError;

/// JWT lifetime: 24 hours.
const TOKEN_TTL_SECS: u64 = 86_400;

/// Claims embedded inside the agent JWT.
///
/// Carried as an opaque string in the protobuf `RegisterResponse.token` field.
/// `grpc-listener` decodes and validates this on every authenticated RPC.
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeClaims {
    /// UUID assigned at enrollment. Used to identify the agent without a DB lookup.
    pub node_id: String,
    /// Standard JWT expiry — unix timestamp seconds.
    pub exp: usize,
}

/// Signs a 24-hour HMAC-SHA256 JWT for `node_id`.
///
/// # Errors
///
/// Returns `NodeEnrollmentError::Clock` if the system clock is before UNIX epoch.
/// Returns `NodeEnrollmentError::TokenSign` if encoding fails (empty secret, etc.).
pub fn sign_token(node_id: &str, secret: &[u8]) -> Result<String, NodeEnrollmentError> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| NodeEnrollmentError::Clock(format!("system clock before UNIX epoch: {e}")))?;

    #[allow(clippy::cast_possible_truncation)]
    let exp = now_secs.as_secs().saturating_add(TOKEN_TTL_SECS) as usize;

    let claims = NodeClaims {
        node_id: node_id.to_string(),
        exp,
    };

    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, Validation, decode};

    const SECRET: &[u8] = b"test-secret-long-enough-for-hs256-validation";
    const NODE_ID: &str = "a1b2c3d4-0001-0000-0000-000000000001";

    #[test]
    fn signed_token_decodes_with_same_secret() {
        let token = sign_token(NODE_ID, SECRET).expect("sign_token failed");
        let data = decode::<NodeClaims>(
            &token,
            &DecodingKey::from_secret(SECRET),
            &Validation::default(),
        )
        .expect("decode failed");
        assert_eq!(data.claims.node_id, NODE_ID);
    }

    #[test]
    fn token_carries_correct_node_id() {
        let token = sign_token("other-node", SECRET).expect("sign failed");
        let data = decode::<NodeClaims>(
            &token,
            &DecodingKey::from_secret(SECRET),
            &Validation::default(),
        )
        .expect("decode failed");
        assert_eq!(data.claims.node_id, "other-node");
    }

    #[test]
    fn wrong_secret_fails_decode() {
        let token = sign_token(NODE_ID, SECRET).expect("sign failed");
        let result = decode::<NodeClaims>(
            &token,
            &DecodingKey::from_secret(b"wrong-secret"),
            &Validation::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn exp_is_24h_from_now() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock ok")
            .as_secs() as usize;

        let token = sign_token(NODE_ID, SECRET).expect("sign failed");
        let data = decode::<NodeClaims>(
            &token,
            &DecodingKey::from_secret(SECRET),
            &Validation::default(),
        )
        .expect("decode failed");

        let expected_min = before + TOKEN_TTL_SECS as usize - 5;
        let expected_max = before + TOKEN_TTL_SECS as usize + 5;
        assert!(
            data.claims.exp >= expected_min && data.claims.exp <= expected_max,
            "exp={} expected in [{expected_min}, {expected_max}]",
            data.claims.exp
        );
    }
}
