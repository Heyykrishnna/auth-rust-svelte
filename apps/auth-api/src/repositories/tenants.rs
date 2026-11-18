use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    Application, ApplicationCredential, ApplicationDomain, ApplicationEnvironment, ApplicationId,
    CredentialType, Developer, DeveloperId, DeveloperWorkspace, EnrollmentPolicy, EnvironmentId,
    EnvironmentKind, WorkspaceId, WorkspaceRole,
};
use crate::repositories::redis_ephemeral::normalize_email;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkspaceWithRole {
    pub id: WorkspaceId,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub logo_version: Option<Uuid>,
}

pub enum WorkspaceLogoUpdate {
    Replace(Vec<u8>),
    Remove,
}

pub async fn create_developer<'e, E>(
    executor: E,
    email: &str,
    display_name: &str,
    password_hash: Option<&str>,
) -> Result<Developer, AppError>
where
    E: PgExecutor<'e>,
{
    let developer = sqlx::query_as::<_, Developer>(
        r#"
        INSERT INTO developers (email, display_name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, email, password_hash, display_name, email_verified, status, created_at, updated_at
        "#,
    )
    .bind(normalize_email(email))
    .bind(display_name)
    .bind(password_hash)
    .fetch_one(executor)
    .await?;

    Ok(developer)
}

pub async fn find_developer_by_email<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<Developer>, AppError>
where
    E: PgExecutor<'e>,
{
    let developer = sqlx::query_as::<_, Developer>(
        r#"
        SELECT id, email, password_hash, display_name, email_verified, status, created_at, updated_at
        FROM developers
        WHERE LOWER(email) = $1
        LIMIT 1
        "#,
    )
    .bind(normalize_email(email))
    .fetch_optional(executor)
    .await?;

    Ok(developer)
}

pub async fn find_developer_by_id<'e, E>(
    executor: E,
    developer_id: DeveloperId,
) -> Result<Option<Developer>, AppError>
where
    E: PgExecutor<'e>,
{
    let developer = sqlx::query_as::<_, Developer>(
        r#"
        SELECT id, email, password_hash, display_name, email_verified, status, created_at, updated_at
        FROM developers
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(developer_id)
    .fetch_optional(executor)
    .await?;

    Ok(developer)
}

pub async fn create_workspace<'e, E>(
    executor: E,
    slug: &str,
    name: &str,
) -> Result<DeveloperWorkspace, AppError>
where
    E: PgExecutor<'e>,
{
    let workspace = sqlx::query_as::<_, DeveloperWorkspace>(
        r#"
        INSERT INTO developer_workspaces (slug, name)
        VALUES ($1, $2)
        RETURNING id, slug, name, status, created_at, updated_at
        "#,
    )
    .bind(slug.trim().to_lowercase())
    .bind(name.trim())
    .fetch_one(executor)
    .await?;

    Ok(workspace)
}

pub async fn add_workspace_member<'e, E>(
    executor: E,
    workspace_id: WorkspaceId,
    developer_id: DeveloperId,
    role: WorkspaceRole,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        INSERT INTO developer_workspace_members (workspace_id, developer_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (workspace_id, developer_id) DO UPDATE SET role = EXCLUDED.role
        "#,
    )
    .bind(workspace_id)
    .bind(developer_id)
    .bind(role.as_str())
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn update_workspace_profile<'e, E>(
    executor: E,
    workspace_id: WorkspaceId,
    developer_id: DeveloperId,
    name: Option<&str>,
    logo: Option<WorkspaceLogoUpdate>,
) -> Result<Option<WorkspaceWithRole>, AppError>
where
    E: PgExecutor<'e>,
{
    let change_logo = logo.is_some();
    let (logo_jpeg, logo_version) = match logo {
        Some(WorkspaceLogoUpdate::Replace(bytes)) => (Some(bytes), Some(Uuid::new_v4())),
        _ => (None, None),
    };
    let workspace = sqlx::query_as::<_, WorkspaceWithRole>(
        r#"
        WITH editor AS (
            SELECT workspace_id, role
            FROM developer_workspace_members
            WHERE workspace_id = $1 AND developer_id = $2 AND role IN ('owner', 'admin')
            FOR SHARE
        )
        UPDATE developer_workspaces AS workspace
        SET name = COALESCE($3, workspace.name),
            logo_jpeg = CASE WHEN $4 THEN $5 ELSE workspace.logo_jpeg END,
            logo_version = CASE WHEN $4 THEN $6 ELSE workspace.logo_version END
        FROM editor
        WHERE workspace.id = editor.workspace_id AND workspace.status != 'deleted'
        RETURNING workspace.id, workspace.slug, workspace.name, workspace.status,
                  editor.role, workspace.created_at, workspace.logo_version
        "#,
    )
    .bind(workspace_id)
    .bind(developer_id)
    .bind(name)
    .bind(change_logo)
    .bind(logo_jpeg)
    .bind(logo_version)
    .fetch_optional(executor)
    .await?;

    Ok(workspace)
}

pub async fn workspace_logo<'e, E>(
    executor: E,
    workspace_id: WorkspaceId,
    developer_id: DeveloperId,
) -> Result<Option<Vec<u8>>, AppError>
where
    E: PgExecutor<'e>,
{
    let logo = sqlx::query_scalar(
        r#"
        SELECT workspace.logo_jpeg
        FROM developer_workspaces workspace
        JOIN developer_workspace_members member ON member.workspace_id = workspace.id
        WHERE workspace.id = $1 AND member.developer_id = $2
          AND workspace.status != 'deleted' AND workspace.logo_jpeg IS NOT NULL
        "#,
    )
    .bind(workspace_id)
    .bind(developer_id)
    .fetch_optional(executor)
    .await?;

    Ok(logo)
}

pub async fn list_developer_workspaces<'e, E>(
    executor: E,
    developer_id: DeveloperId,
) -> Result<Vec<WorkspaceWithRole>, AppError>
where
    E: PgExecutor<'e>,
{
    let workspaces = sqlx::query_as::<_, WorkspaceWithRole>(
        r#"
        SELECT dw.id, dw.slug, dw.name, dw.status, dwm.role, dw.created_at, dw.logo_version
        FROM developer_workspaces dw
        JOIN developer_workspace_members dwm ON dwm.workspace_id = dw.id
        WHERE dwm.developer_id = $1 AND dw.status != 'deleted'
        ORDER BY dw.created_at ASC
        "#,
    )
    .bind(developer_id)
    .fetch_all(executor)
    .await?;

    Ok(workspaces)
}

pub async fn get_workspace_member_role<'e, E>(
    executor: E,
    workspace_id: WorkspaceId,
    developer_id: DeveloperId,
) -> Result<Option<String>, AppError>
where
    E: PgExecutor<'e>,
{
    let role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role
        FROM developer_workspace_members
        WHERE workspace_id = $1 AND developer_id = $2
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(developer_id)
    .fetch_optional(executor)
    .await?;

    Ok(role)
}

pub async fn create_application<'e, E>(
    executor: E,
    workspace_id: WorkspaceId,
    slug: &str,
    name: &str,
) -> Result<Application, AppError>
where
    E: PgExecutor<'e>,
{
    let app = sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (workspace_id, slug, name)
        VALUES ($1, $2, $3)
        RETURNING id, workspace_id, slug, name, status, metadata, created_at, updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(slug.trim().to_lowercase())
    .bind(name.trim())
    .fetch_one(executor)
    .await?;

    Ok(app)
}

pub async fn find_application_by_id<'e, E>(
    executor: E,
    app_id: ApplicationId,
) -> Result<Option<Application>, AppError>
where
    E: PgExecutor<'e>,
{
    let app = sqlx::query_as::<_, Application>(
        r#"
        SELECT id, workspace_id, slug, name, status, metadata, created_at, updated_at
        FROM applications
        WHERE id = $1 AND status != 'deleted'
        LIMIT 1
        "#,
    )
    .bind(app_id)
    .fetch_optional(executor)
    .await?;

    Ok(app)
}

pub async fn list_developer_applications<'e, E>(
    executor: E,
    developer_id: DeveloperId,
    workspace_id: Option<WorkspaceId>,
) -> Result<Vec<Application>, AppError>
where
    E: PgExecutor<'e>,
{
    let apps = sqlx::query_as::<_, Application>(
        r#"
        SELECT a.id, a.workspace_id, a.slug, a.name, a.status, a.metadata, a.created_at, a.updated_at
        FROM applications a
        JOIN developer_workspace_members dwm ON dwm.workspace_id = a.workspace_id
        WHERE dwm.developer_id = $1
          AND a.status != 'deleted'
          AND ($2::uuid IS NULL OR a.workspace_id = $2)
        ORDER BY a.created_at DESC
        "#,
    )
    .bind(developer_id)
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;

    Ok(apps)
}

pub async fn update_application<'e, E>(
    executor: E,
    app_id: ApplicationId,
    name: &str,
    status: &str,
    metadata: serde_json::Value,
) -> Result<Application, AppError>
where
    E: PgExecutor<'e>,
{
    let app = sqlx::query_as::<_, Application>(
        r#"
        UPDATE applications
        SET name = $2, status = $3, metadata = $4, updated_at = NOW()
        WHERE id = $1 AND status != 'deleted'
        RETURNING id, workspace_id, slug, name, status, metadata, created_at, updated_at
        "#,
    )
    .bind(app_id)
    .bind(name)
    .bind(status)
    .bind(metadata)
    .fetch_one(executor)
    .await?;

    Ok(app)
}

pub async fn soft_delete_application<'e, E>(
    executor: E,
    app_id: ApplicationId,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query("UPDATE applications SET status = 'deleted', updated_at = NOW() WHERE id = $1")
        .bind(app_id)
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn create_environment<'e, E>(
    executor: E,
    application_id: ApplicationId,
    kind: EnvironmentKind,
    slug: &str,
    enrollment_policy: EnrollmentPolicy,
) -> Result<ApplicationEnvironment, AppError>
where
    E: PgExecutor<'e>,
{
    let env = sqlx::query_as::<_, ApplicationEnvironment>(
        r#"
        INSERT INTO application_environments (application_id, kind, slug, enrollment_policy)
        VALUES ($1, $2, $3, $4)
        RETURNING id, application_id, kind, slug, enrollment_policy, session_ttl_secs, access_token_ttl_secs, require_email_verification, require_mfa, branding, settings, status, created_at, updated_at
        "#,
    )
    .bind(application_id)
    .bind(kind.as_str())
    .bind(slug.trim().to_lowercase())
    .bind(enrollment_policy.as_str())
    .fetch_one(executor)
    .await?;

    Ok(env)
}

pub async fn list_application_environments<'e, E>(
    executor: E,
    application_id: ApplicationId,
) -> Result<Vec<ApplicationEnvironment>, AppError>
where
    E: PgExecutor<'e>,
{
    let envs = sqlx::query_as::<_, ApplicationEnvironment>(
        r#"
        SELECT id, application_id, kind, slug, enrollment_policy, session_ttl_secs, access_token_ttl_secs, require_email_verification, require_mfa, branding, settings, status, created_at, updated_at
        FROM application_environments
        WHERE application_id = $1 AND status != 'deleted'
        ORDER BY kind ASC
        "#,
    )
    .bind(application_id)
    .fetch_all(executor)
    .await?;

    Ok(envs)
}

pub async fn find_environment_by_app_and_kind<'e, E>(
    executor: E,
    application_id: ApplicationId,
    kind: EnvironmentKind,
) -> Result<Option<ApplicationEnvironment>, AppError>
where
    E: PgExecutor<'e>,
{
    let env = sqlx::query_as::<_, ApplicationEnvironment>(
        r#"
        SELECT id, application_id, kind, slug, enrollment_policy, session_ttl_secs, access_token_ttl_secs, require_email_verification, require_mfa, branding, settings, status, created_at, updated_at
        FROM application_environments
        WHERE application_id = $1 AND kind = $2 AND status != 'deleted'
        LIMIT 1
        "#,
    )
    .bind(application_id)
    .bind(kind.as_str())
    .fetch_optional(executor)
    .await?;

    Ok(env)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_environment<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    enrollment_policy: &str,
    session_ttl_secs: i32,
    access_token_ttl_secs: i32,
    require_email_verification: bool,
    require_mfa: bool,
    branding: serde_json::Value,
    settings: serde_json::Value,
) -> Result<ApplicationEnvironment, AppError>
where
    E: PgExecutor<'e>,
{
    let env = sqlx::query_as::<_, ApplicationEnvironment>(
        r#"
        UPDATE application_environments
        SET enrollment_policy = $2,
            session_ttl_secs = $3,
            access_token_ttl_secs = $4,
            require_email_verification = $5,
            require_mfa = $6,
            branding = $7,
            settings = $8,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, application_id, kind, slug, enrollment_policy, session_ttl_secs, access_token_ttl_secs, require_email_verification, require_mfa, branding, settings, status, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(enrollment_policy)
    .bind(session_ttl_secs)
    .bind(access_token_ttl_secs)
    .bind(require_email_verification)
    .bind(require_mfa)
    .bind(branding)
    .bind(settings)
    .fetch_one(executor)
    .await?;

    Ok(env)
}

pub async fn seed_default_environment_roles(
    tx: &mut crate::repositories::db::PgTx<'_>,
    environment_id: EnvironmentId,
) -> Result<(), AppError> {
    let user_role_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO environment_roles (environment_id, slug, name, scope, description, is_system)
        VALUES ($1, 'user', 'User', 'application', 'Default authenticated user role', TRUE)
        ON CONFLICT (environment_id, slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .bind(environment_id)
    .fetch_one(&mut **tx)
    .await?;

    let admin_role_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO environment_roles (environment_id, slug, name, scope, description, is_system)
        VALUES ($1, 'admin', 'Administrator', 'application', 'Application administrator role', TRUE)
        ON CONFLICT (environment_id, slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .bind(environment_id)
    .fetch_one(&mut **tx)
    .await?;

    let perms = [
        ("profile.read", "Read own profile", true),
        ("profile.write", "Update own profile", true),
        ("sessions.read", "List own sessions", true),
        ("sessions.delete", "Revoke own sessions", true),
        ("users.read", "List application users", false),
        ("users.delete", "Remove application users", false),
    ];

    for (slug, description, grant_to_user) in perms {
        let perm_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO environment_permissions (environment_id, slug, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (environment_id, slug) DO UPDATE SET description = EXCLUDED.description
            RETURNING id
            "#,
        )
        .bind(environment_id)
        .bind(slug)
        .bind(description)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO environment_role_permissions (environment_id, role_id, permission_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (environment_id, role_id, permission_id) DO NOTHING
            "#,
        )
        .bind(environment_id)
        .bind(admin_role_id)
        .bind(perm_id)
        .execute(&mut **tx)
        .await?;

        if grant_to_user {
            sqlx::query(
                r#"
                INSERT INTO environment_role_permissions (environment_id, role_id, permission_id)
                VALUES ($1, $2, $3)
                ON CONFLICT (environment_id, role_id, permission_id) DO NOTHING
                "#,
            )
            .bind(environment_id)
            .bind(user_role_id)
            .bind(perm_id)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

pub async fn create_publishable_credential<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    key_prefix: &str,
    public_key: &str,
    name: &str,
    created_by: Option<DeveloperId>,
) -> Result<ApplicationCredential, AppError>
where
    E: PgExecutor<'e>,
{
    let credential = sqlx::query_as::<_, ApplicationCredential>(
        r#"
        INSERT INTO application_credentials (environment_id, credential_type, key_prefix, public_key, name, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, environment_id, credential_type, key_prefix, public_key, secret_hash, name, scopes, last_used_at, expires_at, revoked_at, created_by, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(CredentialType::Publishable.as_str())
    .bind(key_prefix)
    .bind(public_key)
    .bind(name)
    .bind(created_by)
    .fetch_one(executor)
    .await?;

    Ok(credential)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_secret_credential<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    key_prefix: &str,
    secret_hash: &str,
    name: &str,
    scopes: &[String],
    created_by: Option<DeveloperId>,
) -> Result<ApplicationCredential, AppError>
where
    E: PgExecutor<'e>,
{
    let credential = sqlx::query_as::<_, ApplicationCredential>(
        r#"
        INSERT INTO application_credentials (environment_id, credential_type, key_prefix, secret_hash, name, scopes, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, environment_id, credential_type, key_prefix, public_key, secret_hash, name, scopes, last_used_at, expires_at, revoked_at, created_by, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(CredentialType::Secret.as_str())
    .bind(key_prefix)
    .bind(secret_hash)
    .bind(name)
    .bind(scopes)
    .bind(created_by)
    .fetch_one(executor)
    .await?;

    Ok(credential)
}

pub async fn list_environment_credentials<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
) -> Result<Vec<ApplicationCredential>, AppError>
where
    E: PgExecutor<'e>,
{
    let creds = sqlx::query_as::<_, ApplicationCredential>(
        r#"
        SELECT id, environment_id, credential_type, key_prefix, public_key, secret_hash, name, scopes, last_used_at, expires_at, revoked_at, created_by, created_at, updated_at
        FROM application_credentials
        WHERE environment_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(environment_id)
    .fetch_all(executor)
    .await?;

    Ok(creds)
}

pub async fn find_credential_by_id<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    credential_id: Uuid,
) -> Result<Option<ApplicationCredential>, AppError>
where
    E: PgExecutor<'e>,
{
    let cred = sqlx::query_as::<_, ApplicationCredential>(
        r#"
        SELECT id, environment_id, credential_type, key_prefix, public_key, secret_hash, name, scopes, last_used_at, expires_at, revoked_at, created_by, created_at, updated_at
        FROM application_credentials
        WHERE environment_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(credential_id)
    .fetch_optional(executor)
    .await?;

    Ok(cred)
}

pub async fn revoke_credential<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    credential_id: Uuid,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        UPDATE application_credentials
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE environment_id = $1 AND id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(credential_id)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn set_credential_expiry<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    credential_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        UPDATE application_credentials
        SET expires_at = $3, updated_at = NOW()
        WHERE environment_id = $1 AND id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(credential_id)
    .bind(expires_at)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn revoke_all_environment_credentials<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        UPDATE application_credentials
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE environment_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn touch_credential_last_used<'e, E>(
    executor: E,
    credential_id: Uuid,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query("UPDATE application_credentials SET last_used_at = NOW() WHERE id = $1")
        .bind(credential_id)
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn find_environment_by_id<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
) -> Result<Option<ApplicationEnvironment>, AppError>
where
    E: PgExecutor<'e>,
{
    let env = sqlx::query_as::<_, ApplicationEnvironment>(
        r#"
        SELECT id, application_id, kind, slug, enrollment_policy, session_ttl_secs, access_token_ttl_secs, require_email_verification, require_mfa, branding, settings, status, created_at, updated_at
        FROM application_environments
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .fetch_optional(executor)
    .await?;

    Ok(env)
}

pub async fn find_active_environment_by_publishable_key<'e, E>(
    executor: E,
    public_key: &str,
) -> Result<Option<ApplicationEnvironment>, AppError>
where
    E: PgExecutor<'e>,
{
    let env = sqlx::query_as::<_, ApplicationEnvironment>(
        r#"
        SELECT ae.id, ae.application_id, ae.kind, ae.slug, ae.enrollment_policy, ae.session_ttl_secs, ae.access_token_ttl_secs, ae.require_email_verification, ae.require_mfa, ae.branding, ae.settings, ae.status, ae.created_at, ae.updated_at
        FROM application_credentials ac
        JOIN application_environments ae ON ae.id = ac.environment_id
        JOIN applications a ON a.id = ae.application_id
        WHERE ac.credential_type = 'publishable'
          AND ac.public_key = $1
          AND ac.revoked_at IS NULL
          AND (ac.expires_at IS NULL OR ac.expires_at > NOW())
          AND ae.status = 'active'
          AND a.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(public_key)
    .fetch_optional(executor)
    .await?;

    Ok(env)
}

#[derive(sqlx::FromRow)]
pub struct SecretAuthLookupRow {
    #[sqlx(flatten)]
    pub environment: ApplicationEnvironment,
    pub credential_id: Uuid,
    pub stored_secret_hash: String,
    pub scopes: Vec<String>,
}

pub async fn find_active_environment_by_secret_hash<'e, E>(
    executor: E,
    secret_hash: &str,
) -> Result<Option<(ApplicationEnvironment, Vec<String>)>, AppError>
where
    E: PgExecutor<'e>,
{
    let row = find_secret_auth_lookup(executor, secret_hash).await?;
    Ok(row.map(|r| (r.environment, r.scopes)))
}

pub async fn find_secret_auth_lookup<'e, E>(
    executor: E,
    secret_hash: &str,
) -> Result<Option<SecretAuthLookupRow>, AppError>
where
    E: PgExecutor<'e>,
{
    let row = sqlx::query_as::<_, SecretAuthLookupRow>(
        r#"
        SELECT
            ae.id, ae.application_id, ae.kind, ae.slug, ae.enrollment_policy,
            ae.session_ttl_secs, ae.access_token_ttl_secs, ae.require_email_verification,
            ae.require_mfa, ae.branding, ae.settings, ae.status, ae.created_at, ae.updated_at,
            ac.id AS credential_id,
            ac.secret_hash AS stored_secret_hash,
            ac.scopes
        FROM application_credentials ac
        JOIN application_environments ae ON ae.id = ac.environment_id
        JOIN applications a ON a.id = ae.application_id
        WHERE ac.credential_type = 'secret'
          AND ac.secret_hash = $1
          AND ac.revoked_at IS NULL
          AND (ac.expires_at IS NULL OR ac.expires_at > NOW())
          AND ae.status = 'active'
          AND a.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(secret_hash)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_domain<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    domain: &str,
    is_verified: bool,
    verification_token: Option<&str>,
    allowed_origins: &[String],
    redirect_uris: &[String],
    post_logout_redirect_uris: &[String],
) -> Result<ApplicationDomain, AppError>
where
    E: PgExecutor<'e>,
{
    let row = sqlx::query_as::<_, ApplicationDomain>(
        r#"
        INSERT INTO application_domains (
            environment_id, domain, is_verified, verification_token,
            allowed_origins, redirect_uris, post_logout_redirect_uris
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (environment_id, domain) DO UPDATE SET
            allowed_origins = EXCLUDED.allowed_origins,
            redirect_uris = EXCLUDED.redirect_uris,
            post_logout_redirect_uris = EXCLUDED.post_logout_redirect_uris,
            updated_at = NOW()
        RETURNING id, environment_id, domain, is_verified, verification_token, allowed_origins, redirect_uris, post_logout_redirect_uris, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(domain)
    .bind(is_verified)
    .bind(verification_token)
    .bind(allowed_origins)
    .bind(redirect_uris)
    .bind(post_logout_redirect_uris)
    .fetch_one(executor)
    .await?;

    Ok(row)
}

pub async fn list_environment_domains<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
) -> Result<Vec<ApplicationDomain>, AppError>
where
    E: PgExecutor<'e>,
{
    let rows = sqlx::query_as::<_, ApplicationDomain>(
        r#"
        SELECT id, environment_id, domain, is_verified, verification_token, allowed_origins, redirect_uris, post_logout_redirect_uris, created_at, updated_at
        FROM application_domains
        WHERE environment_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(environment_id)
    .fetch_all(executor)
    .await?;

    Ok(rows)
}

pub async fn delete_domain<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    domain_id: Uuid,
) -> Result<bool, AppError>
where
    E: PgExecutor<'e>,
{
    let result =
        sqlx::query("DELETE FROM application_domains WHERE environment_id = $1 AND id = $2")
            .bind(environment_id)
            .bind(domain_id)
            .execute(executor)
            .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_environment_allowed_origins<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
) -> Result<Vec<String>, AppError>
where
    E: PgExecutor<'e>,
{
    let origins = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT UNNEST(allowed_origins)
        FROM application_domains
        WHERE environment_id = $1
        "#,
    )
    .bind(environment_id)
    .fetch_all(executor)
    .await?;

    Ok(origins)
}

pub async fn is_origin_allowed_in_any_active_environment<'e, E>(
    executor: E,
    origin: &str,
) -> Result<bool, AppError>
where
    E: PgExecutor<'e>,
{
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM application_domains ad
            JOIN application_environments ae ON ae.id = ad.environment_id
            JOIN applications a ON a.id = ae.application_id
            WHERE $1 = ANY(ad.allowed_origins)
              AND ae.status = 'active'
              AND a.status = 'active'
        )
        "#,
    )
    .bind(origin)
    .fetch_one(executor)
    .await?;

    Ok(exists)
}

pub async fn record_control_audit_log<'e, E>(
    executor: E,
    environment_id: Option<EnvironmentId>,
    developer_id: DeveloperId,
    event: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        INSERT INTO audit_logs (environment_id, actor_type, actor_id, event, resource_type, resource_id, metadata)
        VALUES ($1, 'developer', $2, $3, $4, $5, $6)
        "#,
    )
    .bind(environment_id)
    .bind(developer_id)
    .bind(event)
    .bind(resource_type)
    .bind(resource_id)
    .bind(crate::services::audit::sanitize_audit_metadata(metadata))
    .execute(executor)
    .await?;

    Ok(())
}
