use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::DomainError;

/// JWT Claims for access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,          // user ID
    pub email: String,
    pub display_name: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,          // JWT ID (for revocation)
}

/// JWT Claims for refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: String,          // user ID
    pub session_id: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

/// Token pair returned after successful auth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64, // access token expiry in seconds
}

/// Generate an access + refresh token pair for a user.
pub fn generate_token_pair(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    session_id: Uuid,
    secret: &str,
    access_expiry_secs: u64,
    refresh_expiry_secs: u64,
) -> Result<TokenPair, DomainError> {
    let now = Utc::now().timestamp();

    let access_claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        display_name: display_name.to_string(),
        iat: now,
        exp: now + access_expiry_secs as i64,
        jti: Uuid::new_v4().to_string(),
    };

    let refresh_claims = RefreshTokenClaims {
        sub: user_id.to_string(),
        session_id: session_id.to_string(),
        iat: now,
        exp: now + refresh_expiry_secs as i64,
        jti: Uuid::new_v4().to_string(),
    };

    let key = EncodingKey::from_secret(secret.as_bytes());

    let access_token = encode(&Header::new(Algorithm::HS256), &access_claims, &key)
        .map_err(|e| DomainError::TokenCreation(e.to_string()))?;

    let refresh_token = encode(&Header::new(Algorithm::HS256), &refresh_claims, &key)
        .map_err(|e| DomainError::TokenCreation(e.to_string()))?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: access_expiry_secs,
    })
}

/// Validate and decode an access token.
pub fn validate_access_token(
    token: &str,
    secret: &str,
) -> Result<AccessTokenClaims, DomainError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<AccessTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| DomainError::InvalidToken(e.to_string()))
}

/// Validate and decode a refresh token.
pub fn validate_refresh_token(
    token: &str,
    secret: &str,
) -> Result<RefreshTokenClaims, DomainError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<RefreshTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| DomainError::InvalidToken(e.to_string()))
}
