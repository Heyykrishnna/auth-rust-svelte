use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{
    application::register::hash_token,
    domain::{
        errors::DomainError,
        token::{generate_token_pair, validate_refresh_token, TokenPair},
    },
    infrastructure::{postgres, redis},
    AppState,
};

/// Request payload for token refresh.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Refresh an access token using a valid refresh token.
/// Implements refresh token rotation — old token is revoked, new pair issued.
#[instrument(skip(state, req))]
pub async fn refresh(state: &AppState, req: RefreshRequest) -> Result<TokenPair, DomainError> {
    // Validate the refresh token JWT
    let claims = validate_refresh_token(&req.refresh_token, &state.config.jwt_secret)?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| DomainError::InvalidToken("Invalid user ID in refresh token".to_string()))?;

    let session_id = Uuid::parse_str(&claims.session_id)
        .map_err(|_| DomainError::InvalidToken("Invalid session ID".to_string()))?;

    // Verify session exists in Redis and token hash matches
    let session = redis::get_session(&state.redis, session_id).await
        .map_err(|e| DomainError::Redis(e.to_string()))?
        .ok_or(DomainError::SessionNotFound)?;

    if session.is_expired() {
        redis::delete_session(&state.redis, session_id).await.ok();
        return Err(DomainError::SessionNotFound);
    }

    let token_hash = hash_token(&req.refresh_token);
    if session.refresh_token_hash != token_hash {
        // Possible token reuse attack — invalidate the session
        redis::delete_session(&state.redis, session_id).await.ok();
        return Err(DomainError::InvalidToken("Refresh token reuse detected".to_string()));
    }

    // Fetch user
    let user = postgres::find_user_by_id(&state.db, user_id).await?
        .ok_or(DomainError::UserNotFound)?;

    // Generate new token pair (rotation)
    let new_session_id = Uuid::new_v4();
    let tokens = generate_token_pair(
        user.id,
        &user.email,
        &user.display_name,
        new_session_id,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry_secs,
        state.config.jwt_refresh_expiry_secs,
    )?;

    // Revoke old session, store new one
    redis::delete_session(&state.redis, session_id).await.ok();

    let new_session = crate::domain::session::Session::new(
        user.id,
        hash_token(&tokens.refresh_token),
        Utc::now() + state.config.refresh_token_duration(),
        session.user_agent,
        session.ip_address,
    );
    redis::store_session(&state.redis, &new_session).await
        .map_err(|e| DomainError::Redis(e.to_string()))?;

    info!(user_id = %user.id, "Tokens refreshed successfully");
    Ok(tokens)
}
