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
        // 1. Check cookie first (session_token or access_token)
        let cookie_token = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(|cookie_str| {
                cookie_str.split(';').find_map(|s| {
                    let mut parts = s.trim().splitn(2, '=');
                    let name = parts.next()?;
                    let val = parts.next()?;
                    if name == "session_token" || name == "access_token" {
                        Some(val.to_string())
                    } else {
                        None
                    }
                })
            });

        // 2. Fallback to Authorization: Bearer header
        let token = if let Some(ref t) = cookie_token {
            t.as_str()
        } else if let Some(auth_val) = parts.headers.get("authorization").and_then(|v| v.to_str().ok()) {
            auth_val
                .strip_prefix("Bearer ")
                .ok_or_else(|| AppError::Unauthorized("Invalid authorization scheme".to_string()))?
        } else {
            return Err(AppError::Unauthorized("Authentication required".to_string()));
        };

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
