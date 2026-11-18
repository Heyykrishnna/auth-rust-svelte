use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::collections::HashSet;

use crate::errors::AppError;
use crate::middleware::tenant::{PublishableKeyTenant, TenantContext};
use crate::models::{AppSessionId, AppUserId, EnvironmentId, OrganizationId, UserId};
use crate::repositories::redis_keys;
use crate::services::tokens::verify_tenant_access_token;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedClientUser {
    pub tenant: TenantContext,
    pub user_id: UserId,
    pub app_user_id: AppUserId,
    pub session_id: AppSessionId,
    pub active_organization_id: Option<OrganizationId>,
    pub jti: String,
    pub version: i64,
    pub roles: HashSet<String>,
    pub permissions: HashSet<String>,
}

impl AuthenticatedClientUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    pub fn require_permission(&self, permission: &str) -> Result<(), AppError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "Missing required permission '{permission}'"
            )))
        }
    }
}

pub async fn is_env_token_blacklisted(
    redis: &deadpool_redis::Pool,
    environment_id: EnvironmentId,
    jti: &str,
) -> bool {
    let Ok(mut conn) = redis.get().await else {
        return false;
    };
    let key = redis_keys::env_token_blacklist_key(environment_id, jti);
    let exists: Result<i64, _> = redis::cmd("EXISTS").arg(&key).query_async(&mut conn).await;
    matches!(exists, Ok(n) if n > 0)
}

pub async fn blacklist_env_token(
    redis: &deadpool_redis::Pool,
    environment_id: EnvironmentId,
    jti: &str,
    ttl_secs: u64,
) {
    if ttl_secs == 0 {
        return;
    }
    if let Ok(mut conn) = redis.get().await {
        let key = redis_keys::env_token_blacklist_key(environment_id, jti);
        let _: Result<(), _> = redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl_secs)
            .arg("1")
            .query_async(&mut conn)
            .await;
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedClientUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(existing) = parts.extensions.get::<AuthenticatedClientUser>() {
            return Ok(existing.clone());
        }

        let PublishableKeyTenant(tenant) =
            PublishableKeyTenant::from_request_parts(parts, state).await?;

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| AppError::Unauthorized("Invalid Authorization scheme".to_string()))?;

        let claims =
            verify_tenant_access_token(&state.db, &state.config, token, tenant.environment_id)
                .await?;

        if is_env_token_blacklisted(&state.redis, tenant.environment_id, &claims.jti).await {
            return Err(AppError::Unauthorized(
                "Access token has been revoked".to_string(),
            ));
        }

        let user_id = claims
            .sub
            .parse::<UserId>()
            .map_err(|_| AppError::Unauthorized("Invalid user subject".to_string()))?;
        let app_user_id = claims
            .app_user_id
            .parse::<AppUserId>()
            .map_err(|_| AppError::Unauthorized("Invalid app_user_id claim".to_string()))?;
        let session_id = claims
            .sid
            .parse::<AppSessionId>()
            .map_err(|_| AppError::Unauthorized("Invalid sid claim".to_string()))?;
        let active_organization_id = claims
            .org_id
            .as_deref()
            .map(str::parse::<OrganizationId>)
            .transpose()
            .map_err(|_| AppError::Unauthorized("Invalid org_id claim".to_string()))?;

        let authenticated = Self {
            tenant,
            user_id,
            app_user_id,
            session_id,
            active_organization_id,
            jti: claims.jti,
            version: claims.ver,
            roles: claims.roles.into_iter().collect(),
            permissions: claims.permissions.into_iter().collect(),
        };

        parts.extensions.insert(authenticated.clone());
        Ok(authenticated)
    }
}
