use tracing::{info, instrument};
use uuid::Uuid;

use crate::{
    domain::{errors::DomainError, token::validate_access_token},
    infrastructure::redis,
    AppState,
};

/// Logout a user: invalidate the current session in Redis.
#[instrument(skip(state, access_token))]
pub async fn logout(state: &AppState, access_token: &str) -> Result<(), DomainError> {
    let claims = validate_access_token(access_token, &state.config.jwt_secret)?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| DomainError::InvalidToken("Invalid user ID in token".to_string()))?;

    // Invalidate the token by blacklisting the jti in Redis
    let ttl_secs = (claims.exp - chrono::Utc::now().timestamp()).max(0) as u64;
    redis::blacklist_token(&state.redis, &claims.jti, ttl_secs).await
        .map_err(|e| DomainError::Redis(e.to_string()))?;

    info!(user_id = %user_id, "User logged out successfully");
    Ok(())
}
