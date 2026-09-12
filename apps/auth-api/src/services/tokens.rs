use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AccessTokenClaims, EmailVerificationClaims, RefreshTokenClaims, TokenPair};

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_token_pair(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    session_id: Uuid,
    secret: &str,
    access_expiry_secs: u64,
    refresh_expiry_secs: u64,
) -> Result<TokenPair, AppError> {
    let default_roles = vec!["user".to_string()];
    let default_permissions = vec![
        "profile.read".to_string(),
        "profile.write".to_string(),
        "sessions.read".to_string(),
        "sessions.delete".to_string(),
    ];
    generate_token_pair_with_roles_and_permissions(
        user_id,
        email,
        display_name,
        default_roles,
        default_permissions,
        session_id,
        secret,
        access_expiry_secs,
        refresh_expiry_secs,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_token_pair_with_roles_and_permissions(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    session_id: Uuid,
    secret: &str,
    access_expiry_secs: u64,
    refresh_expiry_secs: u64,
) -> Result<TokenPair, AppError> {
    let now = Utc::now().timestamp();

    let access_claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        display_name: display_name.to_string(),
        roles,
        permissions,
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
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    let refresh_token = encode(&Header::new(Algorithm::HS256), &refresh_claims, &key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: access_expiry_secs,
    })
}

pub fn validate_access_token(token: &str, secret: &str) -> Result<AccessTokenClaims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<AccessTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn validate_refresh_token(token: &str, secret: &str) -> Result<RefreshTokenClaims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<RefreshTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn generate_email_verification_token(
    user_id: Uuid,
    email: &str,
    secret: &str,
    expiry_secs: u64,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let claims = EmailVerificationClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        iat: now,
        exp: now + expiry_secs as i64,
    };

    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::new(Algorithm::HS256), &claims, &key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))
}

pub fn validate_email_verification_token(
    token: &str,
    secret: &str,
) -> Result<EmailVerificationClaims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<EmailVerificationClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}
