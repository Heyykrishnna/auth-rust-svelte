use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::errors::AppError;
use crate::models::{OAuthAccount, SsoSession, SsoSessionId, UserEmail, UserEmailId, UserId};
use crate::repositories::redis_ephemeral::normalize_email;

pub async fn create_user_email<'e, E>(
    executor: E,
    user_id: UserId,
    email: &str,
    verified_at: Option<DateTime<Utc>>,
    is_primary: bool,
) -> Result<UserEmail, AppError>
where
    E: PgExecutor<'e>,
{
    let normalized = normalize_email(email);
    let record = sqlx::query_as::<_, UserEmail>(
        r#"
        INSERT INTO user_emails (user_id, email, verified_at, is_primary)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, email, verified_at, is_primary, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(normalized)
    .bind(verified_at)
    .bind(is_primary)
    .fetch_one(executor)
    .await?;

    Ok(record)
}

pub async fn verify_user_email<'e, E>(
    executor: E,
    email_id: UserEmailId,
) -> Result<UserEmail, AppError>
where
    E: PgExecutor<'e>,
{
    let record = sqlx::query_as::<_, UserEmail>(
        r#"
        UPDATE user_emails
        SET verified_at = COALESCE(verified_at, NOW()), updated_at = NOW()
        WHERE id = $1
        RETURNING id, user_id, email, verified_at, is_primary, created_at, updated_at
        "#,
    )
    .bind(email_id)
    .fetch_one(executor)
    .await?;

    Ok(record)
}

pub async fn find_verified_email<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserEmail>, AppError>
where
    E: PgExecutor<'e>,
{
    let normalized = normalize_email(email);
    let record = sqlx::query_as::<_, UserEmail>(
        r#"
        SELECT id, user_id, email, verified_at, is_primary, created_at, updated_at
        FROM user_emails
        WHERE LOWER(email) = $1 AND verified_at IS NOT NULL
        LIMIT 1
        "#,
    )
    .bind(normalized)
    .fetch_optional(executor)
    .await?;

    Ok(record)
}

pub async fn list_user_emails<'e, E>(
    executor: E,
    user_id: UserId,
) -> Result<Vec<UserEmail>, AppError>
where
    E: PgExecutor<'e>,
{
    let records = sqlx::query_as::<_, UserEmail>(
        r#"
        SELECT id, user_id, email, verified_at, is_primary, created_at, updated_at
        FROM user_emails
        WHERE user_id = $1
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(executor)
    .await?;

    Ok(records)
}

pub async fn link_oauth_account<'e, E>(
    executor: E,
    user_id: UserId,
    provider: &str,
    provider_user_id: &str,
    email: Option<&str>,
    email_verified: bool,
) -> Result<OAuthAccount, AppError>
where
    E: PgExecutor<'e>,
{
    let normalized_email = email.map(normalize_email);
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, email_verified)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, provider, provider_user_id, email, email_verified, expires_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(provider.trim().to_lowercase())
    .bind(provider_user_id)
    .bind(normalized_email)
    .bind(email_verified)
    .fetch_one(executor)
    .await?;

    Ok(account)
}

pub async fn find_oauth_account<'e, E>(
    executor: E,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OAuthAccount>, AppError>
where
    E: PgExecutor<'e>,
{
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        SELECT id, user_id, provider, provider_user_id, email, email_verified, expires_at, created_at, updated_at
        FROM oauth_accounts
        WHERE provider = $1 AND provider_user_id = $2
        LIMIT 1
        "#,
    )
    .bind(provider.trim().to_lowercase())
    .bind(provider_user_id)
    .fetch_optional(executor)
    .await?;

    Ok(account)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_sso_session<'e, E>(
    executor: E,
    user_id: UserId,
    session_token_hash: &str,
    amr: &[String],
    acr: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<SsoSession, AppError>
where
    E: PgExecutor<'e>,
{
    let sanitized_ip = ip_address.and_then(|raw| {
        raw.trim()
            .parse::<std::net::IpAddr>()
            .ok()
            .map(|ip| ip.to_string())
    });

    let session = sqlx::query_as::<_, SsoSession>(
        r#"
        INSERT INTO sso_sessions (user_id, session_token_hash, amr, acr, user_agent, ip_address, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
        RETURNING id, user_id, session_token_hash, authenticated_at, amr, acr, user_agent, host(ip_address) AS ip_address, expires_at, last_seen_at, revoked_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(session_token_hash)
    .bind(amr)
    .bind(acr)
    .bind(user_agent)
    .bind(sanitized_ip)
    .bind(expires_at)
    .fetch_one(executor)
    .await?;

    Ok(session)
}

pub async fn find_active_sso_session_by_hash<'e, E>(
    executor: E,
    session_token_hash: &str,
) -> Result<Option<SsoSession>, AppError>
where
    E: PgExecutor<'e>,
{
    let session = sqlx::query_as::<_, SsoSession>(
        r#"
        SELECT id, user_id, session_token_hash, authenticated_at, amr, acr, user_agent, host(ip_address) AS ip_address, expires_at, last_seen_at, revoked_at, created_at, updated_at
        FROM sso_sessions
        WHERE session_token_hash = $1 AND revoked_at IS NULL AND expires_at > NOW()
        LIMIT 1
        "#,
    )
    .bind(session_token_hash)
    .fetch_optional(executor)
    .await?;

    Ok(session)
}

pub async fn revoke_sso_session<'e, E>(
    executor: E,
    sso_session_id: SsoSessionId,
) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    sqlx::query("UPDATE sso_sessions SET revoked_at = NOW() WHERE id = $1 AND revoked_at IS NULL")
        .bind(sso_session_id)
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn revoke_all_user_sso_sessions<'e, E>(
    executor: E,
    user_id: UserId,
) -> Result<u64, AppError>
where
    E: PgExecutor<'e>,
{
    let result = sqlx::query(
        "UPDATE sso_sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(executor)
    .await?;

    Ok(result.rows_affected())
}
