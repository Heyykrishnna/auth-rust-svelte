use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;
use validator::Validate;

use crate::{
    domain::{
        errors::DomainError,
        session::Session,
        token::{generate_token_pair, TokenPair},
        user::{User, UserProfile},
    },
    infrastructure::{postgres, redis},
    AppState,
};

/// Request payload for user registration.
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 2, max = 100, message = "Display name must be 2–100 characters"))]
    pub display_name: String,
}

/// Response returned after successful registration.
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserProfile,
    pub tokens: TokenPair,
}

/// Register a new user with email and password.
#[instrument(skip(state, req), fields(email = %req.email))]
pub async fn register(state: &AppState, req: RegisterRequest) -> Result<RegisterResponse, DomainError> {
    req.validate().map_err(|e| DomainError::Internal(e.to_string()))?;

    // Check if email already exists
    let existing = postgres::find_user_by_email(&state.db, &req.email).await?;
    if existing.is_some() {
        return Err(DomainError::EmailAlreadyExists);
    }

    // Hash password with Argon2id
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| DomainError::PasswordHash(e.to_string()))?
        .to_string();

    // Create user entity
    let user = User::new(req.email, req.display_name, Some(password_hash));
    let user = postgres::create_user(&state.db, user).await?;

    info!(user_id = %user.id, "User registered successfully");

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
        None,
        None,
    );
    redis::store_session(&state.redis, &session).await
        .map_err(|e| DomainError::Redis(e.to_string()))?;

    Ok(RegisterResponse {
        user: UserProfile::from(user),
        tokens,
    })
}

/// Hash a token for secure storage (not reversible).
pub(crate) fn hash_token(token: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    token.hash(&mut h);
    format!("{:x}", h.finish())
}
