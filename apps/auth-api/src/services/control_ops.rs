use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{
    AggregateType, AppSessionId, AppUserId, ApplicationId, DeveloperId, DomainEventEnvelope,
    DomainEventType, EnvironmentId, EnvironmentKind, WorkspaceId, WorkspaceRole,
};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::{outbox, tenant_memberships, tenants as tenant_repo};
use crate::services::backend_sync::{self, UpdateBackendUserMetadataInput};
use crate::services::control_plane::{get_environment_for_developer, require_workspace_access};
use crate::services::signing_keys::{self, OAUTH_SECRET_AAD_PURPOSE};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntryView {
    pub id: Uuid,
    pub environment_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub event: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentOverviewResponse {
    pub environment_id: EnvironmentId,
    pub environment_kind: String,
    pub environment_slug: String,
    pub total_users: i64,
    pub active_users: i64,
    pub suspended_users: i64,
    pub unverified_users: i64,
    pub new_users_7d: i64,
    pub active_sessions: i64,
    pub auth_success_24h: i64,
    pub auth_failure_24h: i64,
    pub auth_success_7d: i64,
    pub auth_failure_7d: i64,
    pub active_api_keys: i64,
    pub webhook_endpoints_active: i64,
    pub webhook_deliveries_succeeded_24h: i64,
    pub webhook_deliveries_failed_24h: i64,
    pub webhook_dead_letters: i64,
    pub issuer_url: String,
    pub jwks_uri: String,
    pub publishable_key: Option<String>,
    pub recent_events: Vec<AuditLogEntryView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfigView {
    pub provider: String,
    pub enabled: bool,
    pub use_platform_credentials: bool,
    pub client_id: Option<String>,
    pub has_custom_secret: bool,
    pub scopes: Vec<String>,
    pub callback_url: String,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertOAuthProviderInput {
    pub enabled: bool,
    pub use_platform_credentials: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentUserListItem {
    pub app_user_id: AppUserId,
    pub sub: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub status: String,
    pub external_id: Option<String>,
    pub roles: Vec<String>,
    pub active_sessions_count: i64,
    pub version: i64,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedEnvironmentUsersResponse {
    pub users: Vec<EnvironmentUserListItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSessionItemView {
    pub id: AppSessionId,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub active_organization_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedOAuthProviderView {
    pub provider: String,
    pub email: Option<String>,
    pub linked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentUserDetailResponse {
    pub app_user_id: AppUserId,
    pub sub: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub status: String,
    pub external_id: Option<String>,
    pub public_metadata: serde_json::Value,
    pub private_metadata: serde_json::Value,
    pub version: i64,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub roles: Vec<String>,
    pub available_roles: Vec<String>,
    pub linked_providers: Vec<LinkedOAuthProviderView>,
    pub sessions: Vec<UserSessionItemView>,
    pub audit_logs: Vec<AuditLogEntryView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedAuditLogsResponse {
    pub logs: Vec<AuditLogEntryView>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMemberView {
    pub developer_id: DeveloperId,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct ListEnvironmentUsersFilter {
    pub query: Option<String>,
    pub status: Option<String>,
    pub email_verified: Option<bool>,
    pub role: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ListAuditLogsFilter {
    pub event: Option<String>,
    pub actor_type: Option<String>,
    pub resource_type: Option<String>,
    pub query: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, FromRow)]
struct AuditLogRow {
    id: Uuid,
    environment_id: Option<Uuid>,
    user_id: Option<Uuid>,
    actor_type: String,
    actor_id: Option<Uuid>,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    event: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl From<AuditLogRow> for AuditLogEntryView {
    fn from(r: AuditLogRow) -> Self {
        Self {
            id: r.id,
            environment_id: r.environment_id,
            user_id: r.user_id,
            actor_type: r.actor_type,
            actor_id: r.actor_id,
            resource_type: r.resource_type,
            resource_id: r.resource_id,
            event: r.event,
            ip_address: r.ip_address,
            user_agent: r.user_agent,
            metadata: r.metadata,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct OAuthProviderRow {
    provider: String,
    enabled: bool,
    client_id: Option<String>,
    has_custom_secret: bool,
    scopes: Vec<String>,
    use_platform_credentials: bool,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct EnvironmentUserRow {
    app_user_id: Uuid,
    sub: Uuid,
    email: String,
    email_verified: bool,
    display_name: String,
    avatar_url: Option<String>,
    status: String,
    external_id: Option<String>,
    version: i64,
    joined_at: DateTime<Utc>,
    last_active_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
    active_sessions_count: i64,
}

#[derive(Debug, FromRow)]
struct RoleAssignmentRow {
    application_user_id: Uuid,
    slug: String,
}

#[derive(Debug, FromRow)]
struct UserDetailRow {
    app_user_id: Uuid,
    sub: Uuid,
    email: String,
    email_verified: bool,
    display_name: String,
    avatar_url: Option<String>,
    status: String,
    external_id: Option<String>,
    public_metadata: serde_json::Value,
    private_metadata: serde_json::Value,
    version: i64,
    joined_at: DateTime<Utc>,
    last_active_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UserSessionRow {
    id: Uuid,
    user_agent: Option<String>,
    ip_address: Option<String>,
    active_organization_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct LinkedProviderRow {
    provider: String,
    email: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct WorkspaceMemberRow {
    developer_id: Uuid,
    email: String,
    display_name: String,
    role: String,
    status: String,
    created_at: DateTime<Utc>,
}

pub fn workspace_role_rank(role: &str) -> u8 {
    match role {
        "owner" => 4,
        "admin" => 3,
        "developer" => 2,
        "viewer" => 1,
        _ => 0,
    }
}

pub fn workspace_role_allows(actor_role: &str, required: WorkspaceRole) -> bool {
    workspace_role_rank(actor_role) >= workspace_role_rank(required.as_str())
}

pub fn can_manage_target_role(actor_role: &str, target_role: &str) -> bool {
    if actor_role == "owner" {
        return workspace_role_rank(target_role) > 0;
    }
    if actor_role == "admin" {
        return matches!(target_role, "admin" | "developer" | "viewer");
    }
    false
}

pub fn clamp_pagination(limit: Option<i64>, offset: Option<i64>, max_limit: i64) -> (i64, i64) {
    let clamped_limit = limit.unwrap_or(25).clamp(1, max_limit);
    let clamped_offset = offset.unwrap_or(0).max(0);
    (clamped_limit, clamped_offset)
}

pub fn normalize_oauth_provider(raw: &str) -> Result<String, AppError> {
    let clean = raw.trim().to_lowercase();
    match clean.as_str() {
        "google" | "github" => Ok(clean),
        _ => Err(AppError::Validation(format!(
            "Unsupported OAuth provider '{raw}'; expected 'google' or 'github'"
        ))),
    }
}

fn default_provider_scopes(provider: &str) -> Vec<String> {
    match provider {
        "google" => vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ],
        "github" => vec!["read:user".to_string(), "user:email".to_string()],
        _ => vec![],
    }
}

pub async fn get_environment_overview(
    db: &PgPool,
    config: &AppConfig,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
) -> Result<EnvironmentOverviewResponse, AppError> {
    let env = get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Viewer)
        .await?;

    let (total_users, active_users, suspended_users, unverified_users, new_users_7d): (
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint,
            COUNT(*) FILTER (WHERE au.status = 'active')::bigint,
            COUNT(*) FILTER (WHERE au.status = 'suspended')::bigint,
            COUNT(*) FILTER (WHERE NOT u.email_verified)::bigint,
            COUNT(*) FILTER (WHERE au.joined_at >= NOW() - INTERVAL '7 days')::bigint
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1 AND au.status <> 'deleted'
        "#,
    )
    .bind(env.id)
    .fetch_one(db)
    .await?;

    let (active_sessions,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM app_sessions
        WHERE environment_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
        "#,
    )
    .bind(env.id)
    .fetch_one(db)
    .await?;

    let (auth_success_24h, auth_failure_24h, auth_success_7d, auth_failure_7d): (
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (
                WHERE created_at >= NOW() - INTERVAL '24 hours'
                  AND event IN ('LOGIN_SUCCESS', 'CLIENT_SIGN_IN', 'OIDC_LOGIN_SUCCESS', 'REGISTER_SUCCESS')
            )::bigint,
            COUNT(*) FILTER (
                WHERE created_at >= NOW() - INTERVAL '24 hours'
                  AND event IN ('LOGIN_FAILED', 'ACCOUNT_LOCKED', 'REFRESH_TOKEN_REUSE_DETECTED', 'CLIENT_SIGN_IN_FAILED')
            )::bigint,
            COUNT(*) FILTER (
                WHERE created_at >= NOW() - INTERVAL '7 days'
                  AND event IN ('LOGIN_SUCCESS', 'CLIENT_SIGN_IN', 'OIDC_LOGIN_SUCCESS', 'REGISTER_SUCCESS')
            )::bigint,
            COUNT(*) FILTER (
                WHERE created_at >= NOW() - INTERVAL '7 days'
                  AND event IN ('LOGIN_FAILED', 'ACCOUNT_LOCKED', 'REFRESH_TOKEN_REUSE_DETECTED', 'CLIENT_SIGN_IN_FAILED')
            )::bigint
        FROM audit_logs
        WHERE environment_id = $1
        "#,
    )
    .bind(env.id)
    .fetch_one(db)
    .await?;

    let (active_api_keys,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM application_credentials
        WHERE environment_id = $1
          AND revoked_at IS NULL
          AND (expires_at IS NULL OR expires_at > NOW())
        "#,
    )
    .bind(env.id)
    .fetch_one(db)
    .await?;

    let (webhook_endpoints_active,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM webhook_endpoints
        WHERE environment_id = $1 AND status = 'active'
        "#,
    )
    .bind(env.id)
    .fetch_one(db)
    .await?;

    let (webhook_deliveries_succeeded_24h, webhook_deliveries_failed_24h, webhook_dead_letters): (
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'succeeded' AND created_at >= NOW() - INTERVAL '24 hours')::bigint,
            COUNT(*) FILTER (WHERE status IN ('failed', 'retrying') AND created_at >= NOW() - INTERVAL '24 hours')::bigint,
            COUNT(*) FILTER (WHERE status = 'dead_letter')::bigint
        FROM webhook_deliveries
        WHERE environment_id = $1
        "#,
    )
    .bind(env.id)
    .fetch_one(db)
    .await?;

    let publishable_key: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT public_key
        FROM application_credentials
        WHERE environment_id = $1
          AND credential_type = 'publishable'
          AND revoked_at IS NULL
          AND public_key IS NOT NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(env.id)
    .fetch_optional(db)
    .await?;

    let recent_rows = sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT id, environment_id, user_id, actor_type, actor_id, resource_type, resource_id,
               event, ip_address, user_agent, metadata, created_at
        FROM audit_logs
        WHERE environment_id = $1
        ORDER BY created_at DESC
        LIMIT 15
        "#,
    )
    .bind(env.id)
    .fetch_all(db)
    .await?;

    let issuer = config.issuer_url.trim_end_matches('/').to_string();
    Ok(EnvironmentOverviewResponse {
        environment_id: env.id,
        environment_kind: env.kind,
        environment_slug: env.slug,
        total_users,
        active_users,
        suspended_users,
        unverified_users,
        new_users_7d,
        active_sessions,
        auth_success_24h,
        auth_failure_24h,
        auth_success_7d,
        auth_failure_7d,
        active_api_keys,
        webhook_endpoints_active,
        webhook_deliveries_succeeded_24h,
        webhook_deliveries_failed_24h,
        webhook_dead_letters,
        jwks_uri: format!("{issuer}/.well-known/jwks.json"),
        issuer_url: issuer,
        publishable_key: publishable_key.map(|(pk,)| pk),
        recent_events: recent_rows.into_iter().map(Into::into).collect(),
    })
}

pub async fn list_oauth_providers(
    db: &PgPool,
    config: &AppConfig,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
) -> Result<Vec<OAuthProviderConfigView>, AppError> {
    let env = get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Viewer)
        .await?;

    let rows = sqlx::query_as::<_, OAuthProviderRow>(
        r#"
        SELECT provider, enabled, client_id,
               (client_secret_encrypted IS NOT NULL) AS has_custom_secret,
               scopes, use_platform_credentials, updated_at
        FROM application_oauth_providers
        WHERE environment_id = $1
        "#,
    )
    .bind(env.id)
    .fetch_all(db)
    .await?;

    let mut by_provider: HashMap<String, OAuthProviderRow> =
        rows.into_iter().map(|r| (r.provider.clone(), r)).collect();

    let issuer = config.issuer_url.trim_end_matches('/');
    let mut output = Vec::with_capacity(2);
    for provider in ["google", "github"] {
        let callback_url = format!("{issuer}/v1/client/auth/oauth/{provider}/callback");
        if let Some(row) = by_provider.remove(provider) {
            output.push(OAuthProviderConfigView {
                provider: row.provider,
                enabled: row.enabled,
                use_platform_credentials: row.use_platform_credentials,
                client_id: row.client_id,
                has_custom_secret: row.has_custom_secret,
                scopes: if row.scopes.is_empty() {
                    default_provider_scopes(provider)
                } else {
                    row.scopes
                },
                callback_url,
                updated_at: Some(row.updated_at),
            });
        } else {
            output.push(OAuthProviderConfigView {
                provider: provider.to_string(),
                enabled: false,
                use_platform_credentials: true,
                client_id: None,
                has_custom_secret: false,
                scopes: default_provider_scopes(provider),
                callback_url,
                updated_at: None,
            });
        }
    }

    Ok(output)
}

pub async fn upsert_oauth_provider(
    db: &PgPool,
    config: &AppConfig,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    provider_raw: &str,
    input: UpsertOAuthProviderInput,
) -> Result<OAuthProviderConfigView, AppError> {
    let provider = normalize_oauth_provider(provider_raw)?;
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;

    let clean_client_id = input
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if input.enabled && !input.use_platform_credentials && clean_client_id.is_none() {
        return Err(AppError::Validation(
            "Custom OAuth provider requires a client_id when enabled".to_string(),
        ));
    }

    let scopes = input
        .scopes
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_provider_scopes(&provider));

    let encrypted_secret = match input
        .client_secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(secret_plain) => Some((
            signing_keys::encrypt_envelope(
                secret_plain.as_bytes(),
                &config.signing_key_kek,
                &config.signing_key_kek_version,
                OAUTH_SECRET_AAD_PURPOSE,
            )?,
            config.signing_key_kek_version.clone(),
        )),
        None => None,
    };

    let mut tx = begin_tx(db).await?;
    let row = match encrypted_secret {
        Some((cipher, kek_ver)) => {
            sqlx::query_as::<_, OAuthProviderRow>(
                r#"
                INSERT INTO application_oauth_providers (
                    environment_id, provider, enabled, client_id,
                    client_secret_encrypted, kek_version, scopes, use_platform_credentials
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (environment_id, provider) DO UPDATE
                SET enabled = EXCLUDED.enabled,
                    client_id = EXCLUDED.client_id,
                    client_secret_encrypted = EXCLUDED.client_secret_encrypted,
                    kek_version = EXCLUDED.kek_version,
                    scopes = EXCLUDED.scopes,
                    use_platform_credentials = EXCLUDED.use_platform_credentials,
                    updated_at = NOW()
                RETURNING provider, enabled, client_id,
                          (client_secret_encrypted IS NOT NULL) AS has_custom_secret,
                          scopes, use_platform_credentials, updated_at
                "#,
            )
            .bind(env.id)
            .bind(&provider)
            .bind(input.enabled)
            .bind(&clean_client_id)
            .bind(&cipher)
            .bind(&kek_ver)
            .bind(&scopes)
            .bind(input.use_platform_credentials)
            .fetch_one(&mut *tx)
            .await?
        }
        None => {
            sqlx::query_as::<_, OAuthProviderRow>(
                r#"
                INSERT INTO application_oauth_providers (
                    environment_id, provider, enabled, client_id,
                    scopes, use_platform_credentials
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (environment_id, provider) DO UPDATE
                SET enabled = EXCLUDED.enabled,
                    client_id = COALESCE(EXCLUDED.client_id, application_oauth_providers.client_id),
                    scopes = EXCLUDED.scopes,
                    use_platform_credentials = EXCLUDED.use_platform_credentials,
                    updated_at = NOW()
                RETURNING provider, enabled, client_id,
                          (client_secret_encrypted IS NOT NULL) AS has_custom_secret,
                          scopes, use_platform_credentials, updated_at
                "#,
            )
            .bind(env.id)
            .bind(&provider)
            .bind(input.enabled)
            .bind(&clean_client_id)
            .bind(&scopes)
            .bind(input.use_platform_credentials)
            .fetch_one(&mut *tx)
            .await?
        }
    };

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env.id),
        developer_id,
        "OAUTH_PROVIDER_UPDATED",
        "application_oauth_provider",
        None,
        serde_json::json!({
            "provider": provider,
            "enabled": row.enabled,
            "use_platform_credentials": row.use_platform_credentials
        }),
    )
    .await?;

    commit_tx(tx).await?;

    let issuer = config.issuer_url.trim_end_matches('/');
    Ok(OAuthProviderConfigView {
        callback_url: format!("{issuer}/v1/client/auth/oauth/{}/callback", row.provider),
        provider: row.provider,
        enabled: row.enabled,
        use_platform_credentials: row.use_platform_credentials,
        client_id: row.client_id,
        has_custom_secret: row.has_custom_secret,
        scopes: row.scopes,
        updated_at: Some(row.updated_at),
    })
}

pub async fn list_environment_users(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    filter: ListEnvironmentUsersFilter,
) -> Result<PaginatedEnvironmentUsersResponse, AppError> {
    let env = get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Viewer)
        .await?;
    let (limit, offset) = clamp_pagination(filter.limit, filter.offset, 100);
    let q_like = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));
    let status_filter = filter
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
        .map(str::to_lowercase);
    let role_filter = filter
        .role
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
        .map(str::to_lowercase);

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1
          AND au.status <> 'deleted'
          AND ($2::text IS NULL OR au.status = $2)
          AND ($3::boolean IS NULL OR u.email_verified = $3)
          AND (
              $4::text IS NULL
              OR LOWER(u.email) LIKE $4
              OR LOWER(u.display_name) LIKE $4
              OR LOWER(au.id::text) LIKE $4
              OR LOWER(u.id::text) LIKE $4
              OR LOWER(COALESCE(au.external_id, '')) LIKE $4
          )
          AND (
              $5::text IS NULL
              OR EXISTS (
                  SELECT 1
                  FROM application_user_roles aur
                  JOIN environment_roles r ON r.id = aur.role_id AND r.environment_id = aur.environment_id
                  WHERE aur.environment_id = au.environment_id
                    AND aur.application_user_id = au.id
                    AND LOWER(r.slug) = $5
              )
          )
        "#,
    )
    .bind(env.id)
    .bind(&status_filter)
    .bind(filter.email_verified)
    .bind(&q_like)
    .bind(&role_filter)
    .fetch_one(db)
    .await?;

    let rows = sqlx::query_as::<_, EnvironmentUserRow>(
        r#"
        SELECT
            au.id AS app_user_id,
            u.id AS sub,
            u.email,
            u.email_verified,
            u.display_name,
            u.avatar_url,
            au.status,
            au.external_id,
            au.version,
            au.joined_at,
            au.last_active_at,
            au.updated_at,
            (
                SELECT COUNT(*)::bigint
                FROM app_sessions s
                WHERE s.environment_id = au.environment_id
                  AND s.application_user_id = au.id
                  AND s.revoked_at IS NULL
                  AND s.expires_at > NOW()
            ) AS active_sessions_count
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1
          AND au.status <> 'deleted'
          AND ($2::text IS NULL OR au.status = $2)
          AND ($3::boolean IS NULL OR u.email_verified = $3)
          AND (
              $4::text IS NULL
              OR LOWER(u.email) LIKE $4
              OR LOWER(u.display_name) LIKE $4
              OR LOWER(au.id::text) LIKE $4
              OR LOWER(u.id::text) LIKE $4
              OR LOWER(COALESCE(au.external_id, '')) LIKE $4
          )
          AND (
              $5::text IS NULL
              OR EXISTS (
                  SELECT 1
                  FROM application_user_roles aur
                  JOIN environment_roles r ON r.id = aur.role_id AND r.environment_id = aur.environment_id
                  WHERE aur.environment_id = au.environment_id
                    AND aur.application_user_id = au.id
                    AND LOWER(r.slug) = $5
              )
          )
        ORDER BY au.updated_at DESC, au.id DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(env.id)
    .bind(&status_filter)
    .bind(filter.email_verified)
    .bind(&q_like)
    .bind(&role_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let ids: Vec<Uuid> = rows.iter().map(|r| r.app_user_id).collect();
    let role_rows = if ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, RoleAssignmentRow>(
            r#"
            SELECT aur.application_user_id, r.slug
            FROM application_user_roles aur
            JOIN environment_roles r ON r.id = aur.role_id AND r.environment_id = aur.environment_id
            WHERE aur.environment_id = $1
              AND aur.application_user_id = ANY($2)
            ORDER BY r.slug ASC
            "#,
        )
        .bind(env.id)
        .bind(&ids)
        .fetch_all(db)
        .await?
    };

    let mut roles_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for rr in role_rows {
        roles_map
            .entry(rr.application_user_id)
            .or_default()
            .push(rr.slug);
    }

    let users = rows
        .into_iter()
        .map(|r| EnvironmentUserListItem {
            roles: roles_map.remove(&r.app_user_id).unwrap_or_default(),
            app_user_id: AppUserId::from(r.app_user_id),
            sub: r.sub,
            email: r.email,
            email_verified: r.email_verified,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            status: r.status,
            external_id: r.external_id,
            active_sessions_count: r.active_sessions_count,
            version: r.version,
            joined_at: r.joined_at,
            last_active_at: r.last_active_at,
            updated_at: r.updated_at,
        })
        .collect();

    Ok(PaginatedEnvironmentUsersResponse {
        users,
        total,
        limit,
        offset,
    })
}

pub async fn get_environment_user_detail(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
) -> Result<EnvironmentUserDetailResponse, AppError> {
    let env = get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Viewer)
        .await?;

    let row = sqlx::query_as::<_, UserDetailRow>(
        r#"
        SELECT
            au.id AS app_user_id,
            u.id AS sub,
            u.email,
            u.email_verified,
            u.display_name,
            u.avatar_url,
            au.status,
            au.external_id,
            au.public_metadata,
            au.private_metadata,
            au.version,
            au.joined_at,
            au.last_active_at,
            au.updated_at
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1 AND au.id = $2
        LIMIT 1
        "#,
    )
    .bind(env.id)
    .bind(app_user_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let roles: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT r.slug
        FROM application_user_roles aur
        JOIN environment_roles r ON r.id = aur.role_id AND r.environment_id = aur.environment_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = $2
        ORDER BY r.slug ASC
        "#,
    )
    .bind(env.id)
    .bind(app_user_id)
    .fetch_all(db)
    .await?;

    let available_roles: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT slug
        FROM environment_roles
        WHERE environment_id = $1
        ORDER BY slug ASC
        "#,
    )
    .bind(env.id)
    .fetch_all(db)
    .await?;

    let providers = sqlx::query_as::<_, LinkedProviderRow>(
        r#"
        SELECT provider, email, created_at
        FROM oauth_accounts
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(row.sub)
    .fetch_all(db)
    .await?;

    let sessions = sqlx::query_as::<_, UserSessionRow>(
        r#"
        SELECT id, user_agent, host(ip_address) AS ip_address, active_organization_id,
               created_at, last_used_at, expires_at, revoked_at
        FROM app_sessions
        WHERE environment_id = $1 AND application_user_id = $2
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(env.id)
    .bind(app_user_id)
    .fetch_all(db)
    .await?;

    let audit_rows = sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT id, environment_id, user_id, actor_type, actor_id, resource_type, resource_id,
               event, ip_address, user_agent, metadata, created_at
        FROM audit_logs
        WHERE environment_id = $1
          AND (user_id = $2 OR resource_id = $3)
        ORDER BY created_at DESC
        LIMIT 30
        "#,
    )
    .bind(env.id)
    .bind(row.sub)
    .bind(row.app_user_id)
    .fetch_all(db)
    .await?;

    Ok(EnvironmentUserDetailResponse {
        app_user_id: AppUserId::from(row.app_user_id),
        sub: row.sub,
        email: row.email,
        email_verified: row.email_verified,
        display_name: row.display_name,
        avatar_url: row.avatar_url,
        status: row.status,
        external_id: row.external_id,
        public_metadata: row.public_metadata,
        private_metadata: row.private_metadata,
        version: row.version,
        joined_at: row.joined_at,
        last_active_at: row.last_active_at,
        updated_at: row.updated_at,
        roles: roles.into_iter().map(|(s,)| s).collect(),
        available_roles: available_roles.into_iter().map(|(s,)| s).collect(),
        linked_providers: providers
            .into_iter()
            .map(|p| LinkedOAuthProviderView {
                provider: p.provider,
                email: p.email,
                linked_at: p.created_at,
            })
            .collect(),
        sessions: sessions
            .into_iter()
            .map(|s| UserSessionItemView {
                id: AppSessionId::from(s.id),
                user_agent: s.user_agent,
                ip_address: s.ip_address,
                active_organization_id: s.active_organization_id,
                created_at: s.created_at,
                last_used_at: s.last_used_at,
                expires_at: s.expires_at,
                revoked_at: s.revoked_at,
            })
            .collect(),
        audit_logs: audit_rows.into_iter().map(Into::into).collect(),
    })
}

pub async fn update_environment_user_metadata(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
    input: UpdateBackendUserMetadataInput,
) -> Result<backend_sync::BackendUserSyncItem, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let updated = backend_sync::update_user_metadata(db, env.id, app_user_id, input).await?;
    tenant_repo::record_control_audit_log(
        db,
        Some(env.id),
        developer_id,
        "USER_METADATA_UPDATED",
        "application_user",
        Some(app_user_id.into_inner()),
        serde_json::json!({ "version": updated.version }),
    )
    .await?;
    Ok(updated)
}

pub async fn update_environment_user_role(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
    role_slug: &str,
    action: &str,
) -> Result<Vec<String>, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let clean_slug = role_slug.trim().to_lowercase();
    let is_assign = match action.trim().to_lowercase().as_str() {
        "assign" => true,
        "revoke" => false,
        other => {
            return Err(AppError::Validation(format!(
                "Invalid role action '{other}'; expected 'assign' or 'revoke'"
            )));
        }
    };

    let mut tx = begin_tx(db).await?;
    let membership = tenant_memberships::find_application_user_by_id(&mut *tx, env.id, app_user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let role_row: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT id, slug
        FROM environment_roles
        WHERE environment_id = $1 AND LOWER(slug) = $2
        LIMIT 1
        "#,
    )
    .bind(env.id)
    .bind(&clean_slug)
    .fetch_optional(&mut *tx)
    .await?;

    let (role_id, canonical_slug) = role_row.ok_or_else(|| {
        AppError::Validation(format!("Role '{clean_slug}' not found in environment"))
    })?;

    if is_assign {
        sqlx::query(
            r#"
            INSERT INTO application_user_roles (environment_id, application_user_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (environment_id, application_user_id, role_id) DO NOTHING
            "#,
        )
        .bind(env.id)
        .bind(app_user_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            r#"
            DELETE FROM application_user_roles
            WHERE environment_id = $1 AND application_user_id = $2 AND role_id = $3
            "#,
        )
        .bind(env.id)
        .bind(app_user_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    }

    let (next_version,): (i64,) = sqlx::query_as(
        r#"
        UPDATE application_users
        SET updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING version
        "#,
    )
    .bind(env.id)
    .bind(app_user_id)
    .fetch_one(&mut *tx)
    .await?;

    let event_type = if is_assign {
        DomainEventType::RoleAssigned
    } else {
        DomainEventType::RoleRevoked
    };
    let envelope = DomainEventEnvelope::new(
        event_type,
        Some(env.id.into_inner()),
        AggregateType::ApplicationUser,
        app_user_id.into_inner(),
        next_version,
        serde_json::json!({
            "sub": membership.user_id,
            "app_user_id": app_user_id,
            "role": canonical_slug,
            "version": next_version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, env.id, &envelope).await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env.id),
        developer_id,
        if is_assign {
            "USER_ROLE_ASSIGNED"
        } else {
            "USER_ROLE_REVOKED"
        },
        "application_user",
        Some(app_user_id.into_inner()),
        serde_json::json!({ "role": canonical_slug }),
    )
    .await?;

    let updated_roles: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT r.slug
        FROM application_user_roles aur
        JOIN environment_roles r ON r.id = aur.role_id AND r.environment_id = aur.environment_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = $2
        ORDER BY r.slug ASC
        "#,
    )
    .bind(env.id)
    .bind(app_user_id)
    .fetch_all(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    Ok(updated_roles.into_iter().map(|(s,)| s).collect())
}

pub async fn suspend_environment_user(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
    reason: Option<String>,
) -> Result<backend_sync::BackendUserSyncItem, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let updated =
        backend_sync::suspend_application_user(db, env.id, app_user_id, reason.as_deref()).await?;
    tenant_repo::record_control_audit_log(
        db,
        Some(env.id),
        developer_id,
        "USER_SUSPENDED",
        "application_user",
        Some(app_user_id.into_inner()),
        serde_json::json!({ "reason": reason }),
    )
    .await?;
    Ok(updated)
}

pub async fn restore_environment_user(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
) -> Result<backend_sync::BackendUserSyncItem, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let updated = backend_sync::restore_application_user(db, env.id, app_user_id).await?;
    tenant_repo::record_control_audit_log(
        db,
        Some(env.id),
        developer_id,
        "USER_RESTORED",
        "application_user",
        Some(app_user_id.into_inner()),
        serde_json::json!({ "version": updated.version }),
    )
    .await?;
    Ok(updated)
}

pub async fn trigger_user_password_reset(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
) -> Result<String, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT u.id, u.email
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1 AND au.id = $2
        LIMIT 1
        "#,
    )
    .bind(env.id)
    .bind(app_user_id)
    .fetch_optional(db)
    .await?;

    let (user_id, email) = row.ok_or(AppError::UserNotFound)?;

    tenant_repo::record_control_audit_log(
        db,
        Some(env.id),
        developer_id,
        "PASSWORD_RESET_TRIGGERED_BY_ADMIN",
        "application_user",
        Some(app_user_id.into_inner()),
        serde_json::json!({ "user_id": user_id }),
    )
    .await?;

    Ok(email)
}

pub async fn revoke_all_environment_user_sessions(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    app_user_id: AppUserId,
) -> Result<u64, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let revoked = backend_sync::revoke_all_user_sessions(db, env.id, app_user_id).await?;
    tenant_repo::record_control_audit_log(
        db,
        Some(env.id),
        developer_id,
        "USER_SESSIONS_REVOKED_ALL",
        "application_user",
        Some(app_user_id.into_inner()),
        serde_json::json!({ "revoked_count": revoked }),
    )
    .await?;
    Ok(revoked)
}

pub async fn revoke_environment_session(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    session_id: AppSessionId,
) -> Result<backend_sync::BackendSessionStatusView, AppError> {
    let env =
        get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Developer)
            .await?;
    let revoked = backend_sync::revoke_session(db, env.id, session_id).await?;
    tenant_repo::record_control_audit_log(
        db,
        Some(env.id),
        developer_id,
        "SESSION_REVOKED_BY_ADMIN",
        "app_session",
        Some(session_id.into_inner()),
        serde_json::json!({ "app_user_id": revoked.app_user_id }),
    )
    .await?;
    Ok(revoked)
}

pub async fn list_environment_audit_logs(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    filter: ListAuditLogsFilter,
) -> Result<PaginatedAuditLogsResponse, AppError> {
    let env = get_environment_for_developer(db, developer_id, app_id, kind, WorkspaceRole::Viewer)
        .await?;
    let (limit, offset) = clamp_pagination(filter.limit, filter.offset, 200);
    let event_filter = filter
        .event
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
        .map(str::to_string);
    let actor_filter = filter
        .actor_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
        .map(str::to_lowercase);
    let resource_filter = filter
        .resource_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
        .map(str::to_lowercase);
    let q_like = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM audit_logs
        WHERE environment_id = $1
          AND ($2::text IS NULL OR event = $2)
          AND ($3::text IS NULL OR LOWER(actor_type) = $3)
          AND ($4::text IS NULL OR LOWER(COALESCE(resource_type, '')) = $4)
          AND ($5::timestamptz IS NULL OR created_at >= $5)
          AND ($6::timestamptz IS NULL OR created_at <= $6)
          AND (
              $7::text IS NULL
              OR LOWER(event) LIKE $7
              OR LOWER(COALESCE(resource_type, '')) LIKE $7
              OR LOWER(COALESCE(actor_id::text, '')) LIKE $7
              OR LOWER(COALESCE(resource_id::text, '')) LIKE $7
              OR LOWER(metadata::text) LIKE $7
          )
        "#,
    )
    .bind(env.id)
    .bind(&event_filter)
    .bind(&actor_filter)
    .bind(&resource_filter)
    .bind(filter.from)
    .bind(filter.to)
    .bind(&q_like)
    .fetch_one(db)
    .await?;

    let rows = sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT id, environment_id, user_id, actor_type, actor_id, resource_type, resource_id,
               event, ip_address, user_agent, metadata, created_at
        FROM audit_logs
        WHERE environment_id = $1
          AND ($2::text IS NULL OR event = $2)
          AND ($3::text IS NULL OR LOWER(actor_type) = $3)
          AND ($4::text IS NULL OR LOWER(COALESCE(resource_type, '')) = $4)
          AND ($5::timestamptz IS NULL OR created_at >= $5)
          AND ($6::timestamptz IS NULL OR created_at <= $6)
          AND (
              $7::text IS NULL
              OR LOWER(event) LIKE $7
              OR LOWER(COALESCE(resource_type, '')) LIKE $7
              OR LOWER(COALESCE(actor_id::text, '')) LIKE $7
              OR LOWER(COALESCE(resource_id::text, '')) LIKE $7
              OR LOWER(metadata::text) LIKE $7
          )
        ORDER BY created_at DESC
        LIMIT $8 OFFSET $9
        "#,
    )
    .bind(env.id)
    .bind(&event_filter)
    .bind(&actor_filter)
    .bind(&resource_filter)
    .bind(filter.from)
    .bind(filter.to)
    .bind(&q_like)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(PaginatedAuditLogsResponse {
        logs: rows.into_iter().map(Into::into).collect(),
        total,
        limit,
        offset,
    })
}

pub async fn list_workspace_members(
    db: &PgPool,
    developer_id: DeveloperId,
    workspace_id: WorkspaceId,
) -> Result<Vec<WorkspaceMemberView>, AppError> {
    require_workspace_access(db, developer_id, workspace_id, WorkspaceRole::Viewer).await?;

    let rows = sqlx::query_as::<_, WorkspaceMemberRow>(
        r#"
        SELECT m.developer_id, d.email, d.display_name, m.role, d.status, m.created_at
        FROM developer_workspace_members m
        JOIN developers d ON d.id = m.developer_id
        WHERE m.workspace_id = $1
        ORDER BY m.created_at ASC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| WorkspaceMemberView {
            developer_id: DeveloperId::from(r.developer_id),
            email: r.email,
            display_name: r.display_name,
            role: r.role,
            status: r.status,
            joined_at: r.created_at,
        })
        .collect())
}

pub async fn add_workspace_member_by_email(
    db: &PgPool,
    actor_id: DeveloperId,
    workspace_id: WorkspaceId,
    email: &str,
    role: WorkspaceRole,
) -> Result<WorkspaceMemberView, AppError> {
    let actor_role =
        require_workspace_access(db, actor_id, workspace_id, WorkspaceRole::Admin).await?;
    if !can_manage_target_role(&actor_role, role.as_str()) {
        return Err(AppError::Forbidden(format!(
            "Workspace role '{actor_role}' cannot assign role '{}'",
            role.as_str()
        )));
    }

    let target_dev = tenant_repo::find_developer_by_email(db, email)
        .await?
        .ok_or_else(|| {
            AppError::Validation(format!(
                "No developer account registered with email '{}'",
                email.trim()
            ))
        })?;

    let mut tx = begin_tx(db).await?;
    let existing =
        tenant_repo::get_workspace_member_role(&mut *tx, workspace_id, target_dev.id).await?;
    if existing.is_some() {
        return Err(AppError::Validation(
            "Developer is already a member of this workspace".to_string(),
        ));
    }

    tenant_repo::add_workspace_member(&mut *tx, workspace_id, target_dev.id, role).await?;
    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        actor_id,
        "WORKSPACE_MEMBER_ADDED",
        "developer_workspace",
        Some(workspace_id.into_inner()),
        serde_json::json!({
            "member_id": target_dev.id,
            "role": role.as_str()
        }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(WorkspaceMemberView {
        developer_id: target_dev.id,
        email: target_dev.email,
        display_name: target_dev.display_name,
        role: role.as_str().to_string(),
        status: target_dev.status,
        joined_at: Utc::now(),
    })
}

pub async fn update_workspace_member_role(
    db: &PgPool,
    actor_id: DeveloperId,
    workspace_id: WorkspaceId,
    member_id: DeveloperId,
    new_role: WorkspaceRole,
) -> Result<WorkspaceMemberView, AppError> {
    let actor_role =
        require_workspace_access(db, actor_id, workspace_id, WorkspaceRole::Admin).await?;

    let mut tx = begin_tx(db).await?;
    let current_role = tenant_repo::get_workspace_member_role(&mut *tx, workspace_id, member_id)
        .await?
        .ok_or_else(|| AppError::Validation("Workspace member not found".to_string()))?;

    if !can_manage_target_role(&actor_role, &current_role)
        || !can_manage_target_role(&actor_role, new_role.as_str())
    {
        return Err(AppError::Forbidden(
            "Insufficient permissions to change this member's role".to_string(),
        ));
    }

    if current_role == "owner" && new_role != WorkspaceRole::Owner {
        let (owner_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM developer_workspace_members
            WHERE workspace_id = $1 AND role = 'owner'
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&mut *tx)
        .await?;
        if owner_count <= 1 {
            return Err(AppError::Validation(
                "Workspace must retain at least one owner".to_string(),
            ));
        }
    }

    sqlx::query(
        r#"
        UPDATE developer_workspace_members
        SET role = $3, updated_at = NOW()
        WHERE workspace_id = $1 AND developer_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .bind(new_role.as_str())
    .execute(&mut *tx)
    .await?;

    let updated = sqlx::query_as::<_, WorkspaceMemberRow>(
        r#"
        SELECT m.developer_id, d.email, d.display_name, m.role, d.status, m.created_at
        FROM developer_workspace_members m
        JOIN developers d ON d.id = m.developer_id
        WHERE m.workspace_id = $1 AND m.developer_id = $2
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .fetch_one(&mut *tx)
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        actor_id,
        "WORKSPACE_MEMBER_ROLE_UPDATED",
        "developer_workspace",
        Some(workspace_id.into_inner()),
        serde_json::json!({
            "member_id": member_id,
            "previous_role": current_role,
            "new_role": new_role.as_str()
        }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(WorkspaceMemberView {
        developer_id: DeveloperId::from(updated.developer_id),
        email: updated.email,
        display_name: updated.display_name,
        role: updated.role,
        status: updated.status,
        joined_at: updated.created_at,
    })
}

pub async fn remove_workspace_member(
    db: &PgPool,
    actor_id: DeveloperId,
    workspace_id: WorkspaceId,
    member_id: DeveloperId,
) -> Result<(), AppError> {
    let actor_role =
        require_workspace_access(db, actor_id, workspace_id, WorkspaceRole::Admin).await?;

    let mut tx = begin_tx(db).await?;
    let current_role = tenant_repo::get_workspace_member_role(&mut *tx, workspace_id, member_id)
        .await?
        .ok_or_else(|| AppError::Validation("Workspace member not found".to_string()))?;

    if !can_manage_target_role(&actor_role, &current_role) {
        return Err(AppError::Forbidden(
            "Insufficient permissions to remove this member".to_string(),
        ));
    }

    if current_role == "owner" {
        let (owner_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM developer_workspace_members
            WHERE workspace_id = $1 AND role = 'owner'
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&mut *tx)
        .await?;
        if owner_count <= 1 {
            return Err(AppError::Validation(
                "Cannot remove the last owner of a workspace".to_string(),
            ));
        }
    }

    sqlx::query(
        r#"
        DELETE FROM developer_workspace_members
        WHERE workspace_id = $1 AND developer_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .execute(&mut *tx)
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        actor_id,
        "WORKSPACE_MEMBER_REMOVED",
        "developer_workspace",
        Some(workspace_id.into_inner()),
        serde_json::json!({ "member_id": member_id, "previous_role": current_role }),
    )
    .await?;
    commit_tx(tx).await?;
    Ok(())
}

pub async fn change_developer_password(
    db: &PgPool,
    developer_id: DeveloperId,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    if new_password.len() < 12 || new_password.len() > 128 {
        return Err(AppError::Validation(
            "New password must be between 12 and 128 characters".to_string(),
        ));
    }

    let developer = tenant_repo::find_developer_by_id(db, developer_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let stored_hash = developer.password_hash.as_deref().ok_or_else(|| {
        AppError::Unauthorized("Password authentication is not configured".to_string())
    })?;

    if !crate::services::password_hasher::verify_password_async(
        current_password.to_string(),
        stored_hash.to_string(),
    )
    .await?
    {
        return Err(AppError::Unauthorized(
            "Current password is incorrect".to_string(),
        ));
    }

    let new_hash =
        crate::services::password_hasher::hash_password_async(new_password.to_string()).await?;

    let mut tx = begin_tx(db).await?;
    sqlx::query(
        r#"
        UPDATE developers
        SET password_hash = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(developer_id)
    .bind(&new_hash)
    .execute(&mut *tx)
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        developer_id,
        "DEVELOPER_PASSWORD_CHANGED",
        "developer",
        Some(developer_id.into_inner()),
        serde_json::json!({}),
    )
    .await?;
    commit_tx(tx).await?;
    Ok(())
}
