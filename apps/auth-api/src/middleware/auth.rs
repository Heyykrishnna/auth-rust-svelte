use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::errors::AppError;
use crate::repositories::sessions as session_repo;
use crate::services::tokens::validate_access_token;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub jti: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid authorization scheme".to_string()))?;

        let claims = validate_access_token(token, &state.config.jwt_secret)?;

        let is_blacklisted = session_repo::is_token_blacklisted(&state.redis, &claims.jti)
            .await
            .unwrap_or(false);

        if is_blacklisted {
            return Err(AppError::Unauthorized("Token has been revoked".to_string()));
        }

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid subject in token".to_string()))?;

        Ok(Self {
            user_id,
            email: claims.email,
            display_name: claims.display_name,
            jti: claims.jti,
        })
    }
}
