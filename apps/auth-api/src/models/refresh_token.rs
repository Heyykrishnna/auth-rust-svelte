use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub email: String,
    pub display_name: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: String,
    pub session_id: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerificationClaims {
    pub sub: String,
    pub email: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
