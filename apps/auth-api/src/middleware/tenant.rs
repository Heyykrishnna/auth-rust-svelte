use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{ApplicationId, EnvironmentId, EnvironmentKind};
use crate::repositories::tenants as tenant_repo;
use crate::services::credentials as cred_service;
use crate::services::tokens::hash_token;
use crate::AppState;

pub const PUBLISHABLE_KEY_HEADER: &str = "x-publishable-key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantSource {
    PublishableKey,
    SecretKey { scopes: Vec<String> },
    OidcClient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    pub application_id: ApplicationId,
    pub environment_id: EnvironmentId,
    pub environment_kind: EnvironmentKind,
    pub source: TenantSource,
}

impl TenantContext {
    pub fn has_scope(&self, required: &str) -> bool {
        match &self.source {
            TenantSource::SecretKey { scopes } => scopes.iter().any(|s| s == required || s == "*"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishableKeyTenant(pub TenantContext);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretKeyAuth {
    pub tenant: TenantContext,
    pub credential_id: Uuid,
    pub scopes: Vec<String>,
}

impl SecretKeyAuth {
    pub fn require_scope(&self, required: &str) -> Result<(), AppError> {
        if self.scopes.iter().any(|s| s == required || s == "*") {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "Secret key lacks required scope '{required}'"
            )))
        }
    }
}

pub fn extract_secret_key_bearer(parts: &Parts) -> Option<&str> {
    let raw = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let token = raw.strip_prefix("Bearer ")?.trim();
    if token.starts_with("sk_test_") || token.starts_with("sk_live_") {
        Some(token)
    } else {
        None
    }
}

pub fn extract_publishable_key(parts: &Parts) -> Option<&str> {
    let key = parts
        .headers
        .get(PUBLISHABLE_KEY_HEADER)?
        .to_str()
        .ok()?
        .trim();
    if key.starts_with("pk_test_") || key.starts_with("pk_live_") {
        Some(key)
    } else {
        None
    }
}

async fn resolve_publishable_tenant(
    state: &AppState,
    publishable_key: &str,
) -> Result<TenantContext, AppError> {
    let env = tenant_repo::find_active_environment_by_publishable_key(&state.db, publishable_key)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or inactive publishable key".to_string()))?;

    let kind = EnvironmentKind::from_db_str(&env.kind)
        .ok_or_else(|| AppError::Internal(format!("Invalid environment kind: {}", env.kind)))?;

    if !cred_service::key_matches_environment_kind(publishable_key, kind) {
        return Err(AppError::Forbidden(
            "Publishable key prefix does not match target environment".to_string(),
        ));
    }

    Ok(TenantContext {
        application_id: env.application_id,
        environment_id: env.id,
        environment_kind: kind,
        source: TenantSource::PublishableKey,
    })
}

async fn resolve_secret_auth(
    state: &AppState,
    secret_key: &str,
) -> Result<SecretKeyAuth, AppError> {
    let digest = hash_token(secret_key);
    let lookup = tenant_repo::find_secret_auth_lookup(&state.db, &digest)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or revoked secret key".to_string()))?;

    if !cred_service::constant_time_hash_eq(&lookup.stored_secret_hash, &digest) {
        return Err(AppError::Unauthorized(
            "Invalid or revoked secret key".to_string(),
        ));
    }

    let kind = EnvironmentKind::from_db_str(&lookup.environment.kind).ok_or_else(|| {
        AppError::Internal(format!(
            "Invalid environment kind: {}",
            lookup.environment.kind
        ))
    })?;

    if !cred_service::key_matches_environment_kind(secret_key, kind) {
        return Err(AppError::Forbidden(
            "Secret key environment prefix mismatch".to_string(),
        ));
    }

    if cred_service::should_record_last_used(
        &state.redis,
        lookup.environment.id,
        lookup.credential_id,
    )
    .await
    {
        let db = state.db.clone();
        let cred_id = lookup.credential_id;
        tokio::spawn(async move {
            let _ = tenant_repo::touch_credential_last_used(&db, cred_id).await;
        });
    }

    let tenant = TenantContext {
        application_id: lookup.environment.application_id,
        environment_id: lookup.environment.id,
        environment_kind: kind,
        source: TenantSource::SecretKey {
            scopes: lookup.scopes.clone(),
        },
    };

    Ok(SecretKeyAuth {
        tenant,
        credential_id: lookup.credential_id,
        scopes: lookup.scopes,
    })
}

#[async_trait]
impl FromRequestParts<AppState> for PublishableKeyTenant {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let publishable_key = extract_publishable_key(parts).ok_or_else(|| {
            AppError::Unauthorized("Missing or invalid x-publishable-key header".to_string())
        })?;
        let ctx = resolve_publishable_tenant(state, publishable_key).await?;
        parts.extensions.insert(ctx.clone());
        Ok(PublishableKeyTenant(ctx))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for SecretKeyAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let secret_key = extract_secret_key_bearer(parts).ok_or_else(|| {
            AppError::Unauthorized("Missing or invalid secret key bearer token".to_string())
        })?;
        let auth = resolve_secret_auth(state, secret_key).await?;
        parts.extensions.insert(auth.tenant.clone());
        Ok(auth)
    }
}

#[async_trait]
impl FromRequestParts<AppState> for TenantContext {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(existing) = parts.extensions.get::<TenantContext>() {
            return Ok(existing.clone());
        }

        if let Some(secret_key) = extract_secret_key_bearer(parts) {
            let auth = resolve_secret_auth(state, secret_key).await?;
            parts.extensions.insert(auth.tenant.clone());
            return Ok(auth.tenant);
        }

        if let Some(publishable_key) = extract_publishable_key(parts) {
            let ctx = resolve_publishable_tenant(state, publishable_key).await?;
            parts.extensions.insert(ctx.clone());
            return Ok(ctx);
        }

        Err(AppError::Unauthorized(
            "Missing tenant credential (x-publishable-key or secret key)".to_string(),
        ))
    }
}
