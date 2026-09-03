use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::register::hash_token,
    domain::{
        errors::DomainError,
        session::Session,
        token::{generate_token_pair, TokenPair},
        user::UserProfile,
    },
    infrastructure::{postgres, redis},
    AppState,
};

/// Request payload for login.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

/// Response returned after successful login.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub tokens: TokenPair,
}

/// Login a user with email and password.
#[instrument(skip(state, req), fields(email = %req.email))]
pub async fn login(
    state: &AppState,
    req: LoginRequest,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<LoginResponse, DomainError> {
    req.validate().map_err(|e| DomainError::Internal(e.to_string()))?;

    // Fetch user — use constant time to avoid timing attacks on user existence
    let user = postgres::find_user_by_email(&state.db, &req.email)
        .await?
        .ok_or(DomainError::InvalidCredentials)?;

    // Verify password
    let password_hash = user.password_hash.as_deref()
        .ok_or(DomainError::InvalidCredentials)?; // OIDC-only user has no password

    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| DomainError::InvalidCredentials)?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| {
            warn!(email = %req.email, "Failed login attempt — wrong password");
            DomainError::InvalidCredentials
        })?;

    info!(user_id = %user.id, "User logged in successfully");

    // Generate tokens
    let session_id = Uuid::new_v4();
    let tokens = generate_token_pair(
        user.id,
        &user.email,
        &user.display_name,
        session_id,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry_secs,
        state.config.jwt_refresh_expiry_secs,
    )?;

    // Store session in Redis
    let session = Session::new(
        user.id,
        hash_token(&tokens.refresh_token),
        Utc::now() + state.config.refresh_token_duration(),
        user_agent,
        ip_address,
    );
    redis::store_session(&state.redis, &session).await
        .map_err(|e| DomainError::Redis(e.to_string()))?;

    Ok(LoginResponse {
        user: UserProfile::from(user),
        tokens,
    })
}
