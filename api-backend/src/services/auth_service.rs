//! Business logic service for operator authentication, Argon2 hashing, and JWT token issuance.

use std::sync::Arc;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};

use crate::{
    config::Settings,
    error::AppError,
    models::auth::{Claims, LoginRequest, LoginResponse, UserInfo},
};

/// Service managing operator authentication, credential verification, and JWT session tokens.
#[derive(Clone)]
pub struct AuthService {
    config: Arc<Settings>,
}

impl AuthService {
    /// Creates a new `AuthService` initialized with backend security settings.
    pub fn new(config: Arc<Settings>) -> Self {
        Self { config }
    }

    /// Hashes a raw password string using Argon2id with a secure random salt.
    pub fn hash_password(password: &str) -> anyhow::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .to_string();
        Ok(hash)
    }

    /// Verifies whether a candidate plaintext password matches an existing Argon2 password hash.
    pub fn verify_password(password: &str, password_hash: &str) -> bool {
        if let Ok(parsed_hash) = PasswordHash::new(password_hash) {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok()
        } else {
            false
        }
    }

    /// Authenticates operator credentials and issues a signed Bearer JWT token on success.
    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AppError> {
        let is_valid = if req.username == self.config.admin_default_user {
            if req.password == self.config.admin_default_password {
                true
            } else {
                Self::verify_password(&req.password, &self.config.admin_default_password)
            }
        } else {
            false
        };

        if !is_valid {
            return Err(AppError::Unauthorized("Invalid username or password".into()));
        }

        let now = Utc::now().timestamp();
        let exp = now + self.config.jwt_expiration_secs;

        let claims = Claims {
            sub: req.username.clone(),
            role: "soc_admin".to_string(),
            exp,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalServerError(format!("Token generation failed: {e}")))?;

        Ok(LoginResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.jwt_expiration_secs,
            user: UserInfo {
                username: req.username,
                role: "soc_admin".to_string(),
                permissions: vec![
                    "nodes:read".into(),
                    "nodes:isolate".into(),
                    "alerts:read".into(),
                    "alerts:write".into(),
                    "logs:read".into(),
                ],
            },
        })
    }
}
