use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}
