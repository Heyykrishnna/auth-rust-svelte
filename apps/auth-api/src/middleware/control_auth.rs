use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::errors::AppError;
use crate::models::DeveloperId;
use crate::services::control_auth::{validate_developer_token, DEV_SESSION_COOKIE};
use crate::AppState;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthenticatedDeveloper {
    pub developer_id: DeveloperId,
    pub session_id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedDeveloper {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(dev) = parts.extensions.get::<AuthenticatedDeveloper>() {
            return Ok(dev.clone());
        }

        let cookie_token = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(|cookie_str| {
                cookie_str.split(';').find_map(|entry| {
                    let (name, val) = entry.trim().split_once('=')?;
                    if name.trim() == DEV_SESSION_COOKIE {
                        Some(val.trim().to_string())
                    } else {
                        None
                    }
                })
            });

        let token = if let Some(ref t) = cookie_token {
            t.as_str()
        } else if let Some(auth_val) = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        {
            auth_val
                .strip_prefix("Bearer ")
                .ok_or_else(|| AppError::Unauthorized("Invalid authorization scheme".to_string()))?
        } else {
            return Err(AppError::Unauthorized(
                "Developer authentication required".to_string(),
            ));
        };

        let claims = validate_developer_token(&state.db, token, &state.config).await?;
        let developer_id = claims
            .sub
            .parse::<DeveloperId>()
            .map_err(|_| AppError::Unauthorized("Invalid developer subject".to_string()))?;

        let session_id = claims
            .jti
            .parse::<Uuid>()
            .map_err(|_| AppError::Unauthorized("Invalid developer session".to_string()))?;
        let developer = sqlx::query_as::<_, crate::models::Developer>(
            "SELECT d.* FROM developers d JOIN developer_sessions s ON s.developer_id = d.id
             WHERE d.id = $1 AND s.id = $2 AND s.revoked_at IS NULL AND s.expires_at > NOW()",
        )
        .bind(developer_id)
        .bind(session_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| {
            AppError::Unauthorized("Developer session expired or revoked".to_string())
        })?;

        if developer.status != "active" {
            return Err(AppError::Forbidden(
                "Developer account is suspended or inactive".to_string(),
            ));
        }

        let authenticated = Self {
            developer_id,
            session_id,
            email: developer.email,
            display_name: developer.display_name,
        };
        parts.extensions.insert(authenticated.clone());
        Ok(authenticated)
    }
}
