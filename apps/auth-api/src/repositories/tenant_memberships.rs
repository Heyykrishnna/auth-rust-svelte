use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::errors::AppError;
use crate::models::{
    AppSession, AppSessionId, AppUserId, ApplicationUser, EnvironmentId, Organization,
    OrganizationId, OrganizationMember, SsoSessionId, UserId,
};

pub async fn create_application_user<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    user_id: UserId,
    external_id: Option<&str>,
) -> Result<ApplicationUser, AppError>
where
    E: PgExecutor<'e>,
{
    let membership = sqlx::query_as::<_, ApplicationUser>(
        r#"
        INSERT INTO application_users (environment_id, user_id, external_id)
        VALUES ($1, $2, $3)
        RETURNING id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(user_id)
    .bind(external_id)
    .fetch_one(executor)
    .await?;

    Ok(membership)
}

pub async fn find_application_user_by_user_id<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    user_id: UserId,
) -> Result<Option<ApplicationUser>, AppError>
where
    E: PgExecutor<'e>,
{
    let membership = sqlx::query_as::<_, ApplicationUser>(
        r#"
        SELECT id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        FROM application_users
        WHERE environment_id = $1 AND user_id = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await?;

    Ok(membership)
}

pub async fn list_application_users<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
) -> Result<Vec<ApplicationUser>, AppError>
where
    E: PgExecutor<'e>,
{
    let memberships = sqlx::query_as::<_, ApplicationUser>(
        r#"
        SELECT id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        FROM application_users
        WHERE environment_id = $1
        ORDER BY updated_at ASC, id ASC
        "#,
    )
    .bind(environment_id)
    .fetch_all(executor)
    .await?;

    Ok(memberships)
}

pub async fn update_application_user_metadata<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    public_metadata: serde_json::Value,
    private_metadata: serde_json::Value,
) -> Result<ApplicationUser, AppError>
where
    E: PgExecutor<'e>,
{
    let updated = sqlx::query_as::<_, ApplicationUser>(
        r#"
        UPDATE application_users
        SET public_metadata = $3, private_metadata = $4, updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .bind(public_metadata)
    .bind(private_metadata)
    .fetch_one(executor)
    .await?;

    Ok(updated)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_app_session<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    application_user_id: AppUserId,
    user_id: UserId,
    sso_session_id: Option<SsoSessionId>,
    session_hash: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<AppSession, AppError>
where
    E: PgExecutor<'e>,
{
    let sanitized_ip = ip_address.and_then(|raw| {
        raw.trim()
            .parse::<std::net::IpAddr>()
            .ok()
            .map(|ip| ip.to_string())
    });

    let session = sqlx::query_as::<_, AppSession>(
        r#"
        INSERT INTO app_sessions (environment_id, application_user_id, user_id, sso_session_id, session_hash, user_agent, ip_address, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8)
        RETURNING id, environment_id, application_user_id, user_id, sso_session_id, session_hash, user_agent, host(ip_address) AS ip_address, active_organization_id, expires_at, last_used_at, revoked_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(application_user_id)
    .bind(user_id)
    .bind(sso_session_id)
    .bind(session_hash)
    .bind(user_agent)
    .bind(sanitized_ip)
    .bind(expires_at)
    .fetch_one(executor)
    .await?;

    Ok(session)
}

pub async fn find_app_session_by_id<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    session_id: AppSessionId,
) -> Result<Option<AppSession>, AppError>
where
    E: PgExecutor<'e>,
{
    let session = sqlx::query_as::<_, AppSession>(
        r#"
        SELECT id, environment_id, application_user_id, user_id, sso_session_id, session_hash, user_agent, host(ip_address) AS ip_address, active_organization_id, expires_at, last_used_at, revoked_at, created_at, updated_at
        FROM app_sessions
        WHERE environment_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(session_id)
    .fetch_optional(executor)
    .await?;

    Ok(session)
}

pub async fn revoke_app_session<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    session_id: AppSessionId,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW()
        WHERE environment_id = $1 AND id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(session_id)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn create_organization<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    slug: &str,
    name: &str,
) -> Result<Organization, AppError>
where
    E: PgExecutor<'e>,
{
    let org = sqlx::query_as::<_, Organization>(
        r#"
        INSERT INTO organizations (environment_id, slug, name)
        VALUES ($1, $2, $3)
        RETURNING id, environment_id, slug, name, logo_url, status, metadata, version, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(slug.trim().to_lowercase())
    .bind(name.trim())
    .fetch_one(executor)
    .await?;

    Ok(org)
}

pub async fn add_organization_member<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    organization_id: OrganizationId,
    application_user_id: AppUserId,
) -> Result<OrganizationMember, AppError>
where
    E: PgExecutor<'e>,
{
    let member = sqlx::query_as::<_, OrganizationMember>(
        r#"
        INSERT INTO organization_members (environment_id, organization_id, application_user_id)
        VALUES ($1, $2, $3)
        RETURNING id, environment_id, organization_id, application_user_id, status, version, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(organization_id)
    .bind(application_user_id)
    .fetch_one(executor)
    .await?;

    Ok(member)
}

pub async fn list_organization_members<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    organization_id: OrganizationId,
) -> Result<Vec<OrganizationMember>, AppError>
where
    E: PgExecutor<'e>,
{
    let members = sqlx::query_as::<_, OrganizationMember>(
        r#"
        SELECT id, environment_id, organization_id, application_user_id, status, version, created_at, updated_at
        FROM organization_members
        WHERE environment_id = $1 AND organization_id = $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(environment_id)
    .bind(organization_id)
    .fetch_all(executor)
    .await?;

    Ok(members)
}
