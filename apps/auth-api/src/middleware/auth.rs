use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::collections::HashSet;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{Permission, Role};
use crate::repositories::sessions as session_repo;
use crate::services::tokens::validate_access_token;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub jti: String,
    pub roles: HashSet<Role>,
    pub permissions: HashSet<Permission>,
}

impl AuthenticatedUser {
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn has_any_permission(&self, perms: &[Permission]) -> bool {
        perms.iter().any(|p| self.has_permission(p))
    }

    pub fn has_all_permissions(&self, perms: &[Permission]) -> bool {
        perms.iter().all(|p| self.has_permission(p))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 0. Fast-path: Check if already authenticated by upstream middleware layer
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            return Ok(user.clone());
        }

        // 1. Check cookie first (session_token or access_token)
        let cookie_token = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(|cookie_str| {
                cookie_str.split(';').find_map(|s| {
                    let (name, val) = s.trim().split_once('=')?;
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
        } else if let Some(auth_val) = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
        {
            auth_val
                .strip_prefix("Bearer ")
                .ok_or_else(|| AppError::Unauthorized("Invalid authorization scheme".to_string()))?
        } else {
            return Err(AppError::Unauthorized(
                "Authentication required".to_string(),
            ));
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

        let roles: HashSet<Role> = if claims.roles.is_empty() {
            let mut set = HashSet::new();
            set.insert(Role::User);
            set
        } else {
            claims.roles.into_iter().map(Role::from).collect()
        };

        let permissions: HashSet<Permission> = if claims.permissions.is_empty() {
            let mut set = HashSet::new();
            set.insert(Permission::ProfileRead);
            set.insert(Permission::ProfileWrite);
            set.insert(Permission::SessionsRead);
            set.insert(Permission::SessionsDelete);
            set
        } else {
            claims
                .permissions
                .into_iter()
                .map(Permission::from)
                .collect()
        };

        let authenticated_user = Self {
            user_id,
            email: claims.email,
            display_name: claims.display_name,
            jti: claims.jti,
            roles,
            permissions,
        };

        // Cache in request extensions for downstream handlers/extractors
        parts.extensions.insert(authenticated_user.clone());

        Ok(authenticated_user)
    }
}
