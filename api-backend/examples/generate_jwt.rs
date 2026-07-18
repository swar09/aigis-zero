//! Standalone example demonstrating operator JWT token generation and validation.
//!
//! # Running
//!
//! ```bash
//! cargo run --example generate_jwt
//! ```

use chrono::Utc;
use edr_api_backend::models::auth::Claims;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

fn main() -> anyhow::Result<()> {
    let secret = "super_secret_jwt_key_replace_in_production_32_bytes_min";
    let now = Utc::now().timestamp();
    let exp = now + 86400;

    let claims = Claims {
        sub: "admin".to_string(),
        role: "soc_admin".to_string(),
        exp,
        iat: now,
    };

    println!("Generating Operator JWT token...");
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    println!("Generated Token:\nBearer {token}\n");

    println!("Verifying Token claims...");
    let decoded = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    println!("Verification successful!");
    println!("  Subject: {}", decoded.claims.sub);
    println!("  Role:    {}", decoded.claims.role);
    println!("  Expires: {} Unix Timestamp", decoded.claims.exp);

    Ok(())
}
