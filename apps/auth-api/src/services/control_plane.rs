use chrono::{Duration, Utc};
use deadpool_redis::Pool as RedisPool;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    Application, ApplicationCredential, ApplicationDomain, ApplicationEnvironment, ApplicationId,
    CredentialType, DeveloperId, EnrollmentPolicy, EnvironmentId, EnvironmentKind, WorkspaceId,
    WorkspaceRole,
};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::tenants as tenant_repo;
use crate::services::credentials as cred_service;
use crate::services::domains as domain_service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSummary {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub credential_type: String,
    pub key_prefix: String,
    pub public_key: Option<String>,
    pub name: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<chrono::DateTime<Utc>>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub revoked_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<ApplicationCredential> for CredentialSummary {
    fn from(c: ApplicationCredential) -> Self {
        Self {
            id: c.id,
            environment_id: c.environment_id,
            credential_type: c.credential_type,
            key_prefix: c.key_prefix,
            public_key: c.public_key,
            name: c.name,
            scopes: c.scopes,
            last_used_at: c.last_used_at,
            expires_at: c.expires_at,
            revoked_at: c.revoked_at,
            created_at: c.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedCredentialOutput {
    pub credential: CredentialSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_plaintext: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationWithEnvironments {
    pub application: Application,
    pub environments: Vec<ApplicationEnvironment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedApplicationOutput {
    pub application: Application,
    pub environments: Vec<ApplicationEnvironment>,
    pub development_publishable_key: String,
    pub production_publishable_key: String,
    pub initial_development_secret_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateEnvironmentInput {
    pub enrollment_policy: Option<EnrollmentPolicy>,
    pub session_ttl_secs: Option<i32>,
    pub access_token_ttl_secs: Option<i32>,
    pub require_email_verification: Option<bool>,
    pub require_mfa: Option<bool>,
    pub branding: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}

fn role_rank(role: &str) -> u8 {
    match role {
        "owner" => 4,
        "admin" => 3,
        "developer" => 2,
        "viewer" => 1,
        _ => 0,
    }
}

pub async fn require_workspace_access(
    db: &PgPool,
    developer_id: DeveloperId,
    workspace_id: WorkspaceId,
    min_role: WorkspaceRole,
) -> Result<String, AppError> {
    let role = tenant_repo::get_workspace_member_role(db, workspace_id, developer_id)
        .await?
        .ok_or_else(|| {
            AppError::Forbidden("You do not have access to this workspace".to_string())
        })?;

    if role_rank(&role) < role_rank(min_role.as_str()) {
        return Err(AppError::Forbidden(format!(
            "Workspace role '{}' is insufficient; requires at least '{}'",
            role,
            min_role.as_str()
        )));
    }

    Ok(role)
}

pub async fn require_application_access(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    min_role: WorkspaceRole,
) -> Result<Application, AppError> {
    let app = tenant_repo::find_application_by_id(db, app_id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Application not found or access denied".to_string()))?;

    require_workspace_access(db, developer_id, app.workspace_id, min_role).await?;
    Ok(app)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_application(
    db: &PgPool,
    developer_id: DeveloperId,
    workspace_id: WorkspaceId,
    name: &str,
    slug: &str,
    initial_dev_redirect_uri: Option<&str>,
) -> Result<CreatedApplicationOutput, AppError> {
    require_workspace_access(db, developer_id, workspace_id, WorkspaceRole::Developer).await?;

    let clean_name = name.trim();
    if clean_name.len() < 2 || clean_name.len() > 120 {
        return Err(AppError::Validation(
            "Application name must be between 2 and 120 characters".to_string(),
        ));
    }
    let clean_slug = domain_service::validate_slug(slug)?;

    let validated_redirect = match initial_dev_redirect_uri
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(uri) => Some(domain_service::validate_redirect_uri(
            EnvironmentKind::Development,
            uri,
        )?),
        None => None,
    };

    let (dev_pk_prefix, dev_pk) =
        cred_service::generate_publishable_key(EnvironmentKind::Development);
    let (prod_pk_prefix, prod_pk) =
        cred_service::generate_publishable_key(EnvironmentKind::Production);
    let dev_sk = cred_service::generate_secret_key(EnvironmentKind::Development);
    let default_scopes = vec![
        "users:read".to_string(),
        "users:write".to_string(),
        "sessions:read".to_string(),
        "sessions:revoke".to_string(),
        "sync:read".to_string(),
    ];

    let mut tx = begin_tx(db).await?;
    let app = tenant_repo::create_application(&mut *tx, workspace_id, &clean_slug, clean_name)
        .await
        .map_err(|err| match err {
            AppError::Database(sqlx::Error::Database(db_err))
                if db_err.code().as_deref() == Some("23505") =>
            {
                AppError::Validation(format!("Application slug '{clean_slug}' is already taken"))
            }
            other => other,
        })?;

    let dev_env = tenant_repo::create_environment(
        &mut *tx,
        app.id,
        EnvironmentKind::Development,
        &format!("{clean_slug}-dev"),
        EnrollmentPolicy::ConsentRequired,
    )
    .await?;

    let prod_env = tenant_repo::create_environment(
        &mut *tx,
        app.id,
        EnvironmentKind::Production,
        &format!("{clean_slug}-prod"),
        EnrollmentPolicy::ConsentRequired,
    )
    .await?;

    tenant_repo::seed_default_environment_roles(&mut tx, dev_env.id).await?;
    tenant_repo::seed_default_environment_roles(&mut tx, prod_env.id).await?;

    tenant_repo::create_publishable_credential(
        &mut *tx,
        dev_env.id,
        &dev_pk_prefix,
        &dev_pk,
        "Default Development Publishable Key",
        Some(developer_id),
    )
    .await?;

    tenant_repo::create_publishable_credential(
        &mut *tx,
        prod_env.id,
        &prod_pk_prefix,
        &prod_pk,
        "Default Production Publishable Key",
        Some(developer_id),
    )
    .await?;

    tenant_repo::create_secret_credential(
        &mut *tx,
        dev_env.id,
        &dev_sk.prefix,
        &dev_sk.hash,
        "Initial Development Secret Key",
        &default_scopes,
        Some(developer_id),
    )
    .await?;

    if let Some(redirect_uri) = validated_redirect {
        if let Some(origin) = extract_origin_from_uri(&redirect_uri) {
            let domain = origin
                .split_once("://")
                .map(|(_, rest)| rest.to_string())
                .unwrap_or_else(|| "localhost".to_string());
            tenant_repo::create_domain(
                &mut *tx,
                dev_env.id,
                &domain,
                true,
                None,
                &[origin],
                &[redirect_uri],
                &[],
            )
            .await?;
        }
    }

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(dev_env.id),
        developer_id,
        "APPLICATION_CREATED",
        "application",
        Some(app.id.into_inner()),
        serde_json::json!({ "slug": app.slug, "workspace_id": workspace_id }),
    )
    .await?;

    commit_tx(tx).await?;

    Ok(CreatedApplicationOutput {
        application: app,
        environments: vec![dev_env, prod_env],
        development_publishable_key: dev_pk,
        production_publishable_key: prod_pk,
        initial_development_secret_key: dev_sk.plaintext,
    })
}

fn extract_origin_from_uri(uri: &str) -> Option<String> {
    let (scheme, rest) = uri.split_once("://")?;
    let authority = rest.split('/').next()?;
    Some(format!("{scheme}://{authority}"))
}

pub async fn update_application(
    db: &PgPool,
    redis: &RedisPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    name: Option<&str>,
    status: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<Application, AppError> {
    let existing =
        require_application_access(db, developer_id, app_id, WorkspaceRole::Admin).await?;

    let next_name = match name {
        Some(n) => {
            let t = n.trim();
            if t.len() < 2 || t.len() > 120 {
                return Err(AppError::Validation(
                    "Application name must be between 2 and 120 characters".to_string(),
                ));
            }
            t.to_string()
        }
        None => existing.name.clone(),
    };

    let next_status = match status {
        Some(s @ ("active" | "suspended")) => s.to_string(),
        Some(other) => {
            return Err(AppError::Validation(format!(
                "Invalid application status: {other}"
            )));
        }
        None => existing.status.clone(),
    };

    let next_metadata = metadata.unwrap_or(existing.metadata);

    let mut tx = begin_tx(db).await?;
    let updated =
        tenant_repo::update_application(&mut *tx, app_id, &next_name, &next_status, next_metadata)
            .await?;
    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        developer_id,
        "APPLICATION_UPDATED",
        "application",
        Some(app_id.into_inner()),
        serde_json::json!({ "status": next_status }),
    )
    .await?;
    commit_tx(tx).await?;

    if updated.status != "active" {
        invalidate_all_app_caches(db, redis, app_id).await;
    }

    Ok(updated)
}

pub async fn delete_application(
    db: &PgPool,
    redis: &RedisPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
) -> Result<(), AppError> {
    require_application_access(db, developer_id, app_id, WorkspaceRole::Owner).await?;

    let envs = tenant_repo::list_application_environments(db, app_id).await?;

    let mut tx = begin_tx(db).await?;
    for env in &envs {
        tenant_repo::revoke_all_environment_credentials(&mut *tx, env.id).await?;
    }
    tenant_repo::soft_delete_application(&mut *tx, app_id).await?;
    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        developer_id,
        "APPLICATION_DELETED",
        "application",
        Some(app_id.into_inner()),
        serde_json::json!({ "environments_revoked": envs.len() }),
    )
    .await?;
    commit_tx(tx).await?;

    invalidate_all_app_caches(db, redis, app_id).await;
    Ok(())
}

async fn invalidate_all_app_caches(db: &PgPool, redis: &RedisPool, app_id: ApplicationId) {
    if let Ok(envs) = tenant_repo::list_application_environments(db, app_id).await {
        for env in envs {
            domain_service::invalidate_allowed_origins_cache(redis, env.id).await;
            if let Ok(creds) = tenant_repo::list_environment_credentials(db, env.id).await {
                for cred in creds {
                    if let Some(hash) = cred.secret_hash {
                        cred_service::invalidate_credential_cache(redis, env.id, &hash).await;
                    }
                    if let Some(pk) = cred.public_key {
                        cred_service::invalidate_credential_cache(redis, env.id, &pk).await;
                    }
                }
            }
        }
    }
}

pub async fn get_environment_for_developer(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    min_role: WorkspaceRole,
) -> Result<ApplicationEnvironment, AppError> {
    require_application_access(db, developer_id, app_id, min_role).await?;
    tenant_repo::find_environment_by_app_and_kind(db, app_id, kind)
        .await?
        .ok_or_else(|| AppError::Validation(format!("Environment '{}' not found", kind.as_str())))
}

pub async fn update_environment_settings(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    input: UpdateEnvironmentInput,
) -> Result<ApplicationEnvironment, AppError> {
    let existing =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Admin).await?;

    let enrollment_policy = input
        .enrollment_policy
        .map(|p| p.as_str().to_string())
        .unwrap_or(existing.enrollment_policy);
    let session_ttl_secs = input.session_ttl_secs.unwrap_or(existing.session_ttl_secs);
    let access_token_ttl_secs = input
        .access_token_ttl_secs
        .unwrap_or(existing.access_token_ttl_secs);

    if session_ttl_secs <= 0 {
        return Err(AppError::Validation(
            "session_ttl_secs must be positive".to_string(),
        ));
    }
    if !(60..=86400).contains(&access_token_ttl_secs) {
        return Err(AppError::Validation(
            "access_token_ttl_secs must be between 60 and 86400".to_string(),
        ));
    }

    let require_email_verification = input
        .require_email_verification
        .unwrap_or(existing.require_email_verification);
    let require_mfa = input.require_mfa.unwrap_or(existing.require_mfa);
    let branding = input.branding.unwrap_or(existing.branding);
    let settings = input.settings.unwrap_or(existing.settings);

    let mut tx = begin_tx(db).await?;
    let updated = tenant_repo::update_environment(
        &mut *tx,
        existing.id,
        &enrollment_policy,
        session_ttl_secs,
        access_token_ttl_secs,
        require_email_verification,
        require_mfa,
        branding,
        settings,
    )
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(existing.id),
        developer_id,
        "ENVIRONMENT_UPDATED",
        "application_environment",
        Some(existing.id.into_inner()),
        serde_json::json!({ "kind": kind.as_str(), "enrollment_policy": enrollment_policy }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(updated)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_credential(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    credential_type: CredentialType,
    name: &str,
    scopes: Option<Vec<String>>,
) -> Result<CreatedCredentialOutput, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Admin).await?;
    let clean_name = name.trim();
    if clean_name.is_empty() || clean_name.len() > 120 {
        return Err(AppError::Validation(
            "Credential name must be between 1 and 120 characters".to_string(),
        ));
    }

    let mut tx = begin_tx(db).await?;
    let output = match credential_type {
        CredentialType::Publishable => {
            let (prefix, public_key) = cred_service::generate_publishable_key(kind);
            let created = tenant_repo::create_publishable_credential(
                &mut *tx,
                env.id,
                &prefix,
                &public_key,
                clean_name,
                Some(developer_id),
            )
            .await?;
            tenant_repo::record_control_audit_log(
                &mut *tx,
                Some(env.id),
                developer_id,
                "PUBLISHABLE_KEY_CREATED",
                "application_credential",
                Some(created.id),
                serde_json::json!({ "key_prefix": prefix, "name": clean_name }),
            )
            .await?;
            CreatedCredentialOutput {
                credential: CredentialSummary::from(created),
                secret_plaintext: None,
            }
        }
        CredentialType::Secret => {
            let validated_scopes = cred_service::validate_secret_scopes(
                &scopes.unwrap_or_else(|| vec!["users:read".to_string(), "sync:read".to_string()]),
            )?;
            let generated = cred_service::generate_secret_key(kind);
            let created = tenant_repo::create_secret_credential(
                &mut *tx,
                env.id,
                &generated.prefix,
                &generated.hash,
                clean_name,
                &validated_scopes,
                Some(developer_id),
            )
            .await?;
            tenant_repo::record_control_audit_log(
                &mut *tx,
                Some(env.id),
                developer_id,
                "SECRET_KEY_CREATED",
                "application_credential",
                Some(created.id),
                serde_json::json!({
                    "key_prefix": generated.prefix,
                    "name": clean_name,
                    "scopes": validated_scopes
                }),
            )
            .await?;
            CreatedCredentialOutput {
                credential: CredentialSummary::from(created),
                secret_plaintext: Some(generated.plaintext),
            }
        }
    };

    commit_tx(tx).await?;
    Ok(output)
}

pub async fn rotate_secret_credential(
    db: &PgPool,
    redis: &RedisPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    credential_id: Uuid,
    grace_window_secs: Option<i64>,
) -> Result<CreatedCredentialOutput, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Admin).await?;
    let existing = tenant_repo::find_credential_by_id(db, env.id, credential_id)
        .await?
        .ok_or_else(|| AppError::Validation("Credential not found in environment".to_string()))?;

    if existing.credential_type != CredentialType::Secret.as_str() {
        return Err(AppError::Validation(
            "Only secret credentials support secret rotation".to_string(),
        ));
    }
    if existing.revoked_at.is_some() {
        return Err(AppError::Validation(
            "Cannot rotate an already revoked secret key".to_string(),
        ));
    }

    let grace_secs = grace_window_secs.unwrap_or(0).clamp(0, 86400);
    let generated = cred_service::generate_secret_key(kind);

    let mut tx = begin_tx(db).await?;
    let replacement = tenant_repo::create_secret_credential(
        &mut *tx,
        env.id,
        &generated.prefix,
        &generated.hash,
        &existing.name,
        &existing.scopes,
        Some(developer_id),
    )
    .await?;

    if grace_secs == 0 {
        tenant_repo::revoke_credential(&mut *tx, env.id, existing.id).await?;
    } else {
        let expires_at = Utc::now() + Duration::seconds(grace_secs);
        tenant_repo::set_credential_expiry(&mut *tx, env.id, existing.id, expires_at).await?;
    }

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env.id),
        developer_id,
        "SECRET_KEY_ROTATED",
        "application_credential",
        Some(replacement.id),
        serde_json::json!({
            "previous_credential_id": existing.id,
            "grace_window_secs": grace_secs
        }),
    )
    .await?;
    commit_tx(tx).await?;

    if let Some(old_hash) = existing.secret_hash {
        cred_service::invalidate_credential_cache(redis, env.id, &old_hash).await;
    }

    Ok(CreatedCredentialOutput {
        credential: CredentialSummary::from(replacement),
        secret_plaintext: Some(generated.plaintext),
    })
}

pub async fn revoke_credential(
    db: &PgPool,
    redis: &RedisPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    credential_id: Uuid,
) -> Result<(), AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Admin).await?;
    let existing = tenant_repo::find_credential_by_id(db, env.id, credential_id)
        .await?
        .ok_or_else(|| AppError::Validation("Credential not found in environment".to_string()))?;

    let mut tx = begin_tx(db).await?;
    tenant_repo::revoke_credential(&mut *tx, env.id, credential_id).await?;
    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env.id),
        developer_id,
        "CREDENTIAL_REVOKED",
        "application_credential",
        Some(credential_id),
        serde_json::json!({ "credential_type": existing.credential_type }),
    )
    .await?;
    commit_tx(tx).await?;

    if let Some(hash) = existing.secret_hash {
        cred_service::invalidate_credential_cache(redis, env.id, &hash).await;
    }
    if let Some(pk) = existing.public_key {
        cred_service::invalidate_credential_cache(redis, env.id, &pk).await;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn register_domain(
    db: &PgPool,
    redis: &RedisPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    domain: &str,
    allowed_origins: &[String],
    redirect_uris: &[String],
    post_logout_redirect_uris: &[String],
) -> Result<ApplicationDomain, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;

    let clean_domain = domain_service::validate_domain_name(kind, domain)?;

    let mut clean_origins = Vec::with_capacity(allowed_origins.len());
    for origin in allowed_origins {
        clean_origins.push(domain_service::validate_origin(kind, origin)?);
    }

    let mut clean_redirects = Vec::with_capacity(redirect_uris.len());
    for uri in redirect_uris {
        clean_redirects.push(domain_service::validate_redirect_uri(kind, uri)?);
    }

    let mut clean_post_logout = Vec::with_capacity(post_logout_redirect_uris.len());
    for uri in post_logout_redirect_uris {
        clean_post_logout.push(domain_service::validate_redirect_uri(kind, uri)?);
    }

    let is_verified = matches!(kind, EnvironmentKind::Development);
    let verification_token = if is_verified {
        None
    } else {
        Some(domain_service::generate_dns_verification_token())
    };

    let mut tx = begin_tx(db).await?;
    let created = tenant_repo::create_domain(
        &mut *tx,
        env.id,
        &clean_domain,
        is_verified,
        verification_token.as_deref(),
        &clean_origins,
        &clean_redirects,
        &clean_post_logout,
    )
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env.id),
        developer_id,
        "DOMAIN_REGISTERED",
        "application_domain",
        Some(created.id),
        serde_json::json!({
            "domain": clean_domain,
            "origins_count": clean_origins.len(),
            "redirect_uris_count": clean_redirects.len()
        }),
    )
    .await?;
    commit_tx(tx).await?;

    domain_service::invalidate_allowed_origins_cache(redis, env.id).await;
    Ok(created)
}

pub async fn delete_domain(
    db: &PgPool,
    redis: &RedisPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    domain_id: Uuid,
) -> Result<(), AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;

    let mut tx = begin_tx(db).await?;
    let deleted = tenant_repo::delete_domain(&mut *tx, env.id, domain_id).await?;
    if !deleted {
        return Err(AppError::Validation(
            "Domain not found in environment".to_string(),
        ));
    }
    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env.id),
        developer_id,
        "DOMAIN_DELETED",
        "application_domain",
        Some(domain_id),
        serde_json::json!({ "kind": kind.as_str() }),
    )
    .await?;
    commit_tx(tx).await?;

    domain_service::invalidate_allowed_origins_cache(redis, env.id).await;
    Ok(())
}
