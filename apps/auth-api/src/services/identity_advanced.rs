use chrono::{DateTime, Utc};
use deadpool_redis::Pool as RedisPool;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AggregateType, DomainEventEnvelope, DomainEventType, EnvironmentId, UserId};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::outbox;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmailView {
    pub id: Uuid,
    pub email: String,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedOAuthAccountView {
    pub id: Uuid,
    pub provider: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDataExportResponse {
    pub exported_at: DateTime<Utc>,
    pub user: serde_json::Value,
    pub emails: Vec<UserEmailView>,
    pub oauth_accounts: Vec<LinkedOAuthAccountView>,
    pub memberships: Vec<serde_json::Value>,
    pub organizations: Vec<serde_json::Value>,
    pub recent_audit_logs: Vec<serde_json::Value>,
}

#[derive(Debug, FromRow)]
struct UserEmailRow {
    id: Uuid,
    email: String,
    verified_at: Option<DateTime<Utc>>,
    is_primary: bool,
    created_at: DateTime<Utc>,
}

impl From<UserEmailRow> for UserEmailView {
    fn from(r: UserEmailRow) -> Self {
        Self {
            id: r.id,
            email: r.email,
            verified: r.verified_at.is_some(),
            verified_at: r.verified_at,
            is_primary: r.is_primary,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct OAuthAccountRow {
    id: Uuid,
    provider: String,
    email: Option<String>,
    email_verified: bool,
    created_at: DateTime<Utc>,
}

impl From<OAuthAccountRow> for LinkedOAuthAccountView {
    fn from(r: OAuthAccountRow) -> Self {
        Self {
            id: r.id,
            provider: r.provider,
            email: r.email,
            email_verified: r.email_verified,
            created_at: r.created_at,
        }
    }
}

pub fn generate_email_otp_code() -> String {
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    let num = u32::from_be_bytes(bytes) % 1_000_000;
    format!("{num:06}")
}

pub fn can_remove_email_guard(
    is_primary: bool,
    is_verified: bool,
    verified_email_count: i64,
) -> Result<(), AppError> {
    if is_primary {
        return Err(AppError::Validation(
            "Cannot remove primary email address; promote another verified email first".to_string(),
        ));
    }
    if is_verified && verified_email_count <= 1 {
        return Err(AppError::Validation(
            "Cannot remove the last verified email address on the account".to_string(),
        ));
    }
    Ok(())
}

pub fn can_unlink_oauth_provider_guard(
    has_password: bool,
    linked_provider_count: i64,
) -> Result<(), AppError> {
    if !has_password && linked_provider_count <= 1 {
        return Err(AppError::Validation(
            "Cannot unlink the sole authentication method while no password is set".to_string(),
        ));
    }
    Ok(())
}

fn email_otp_redis_key(user_id: UserId, email: &str) -> String {
    format!("global:email_verify_otp:{user_id}:{email}")
}

pub async fn list_user_emails(
    db: &PgPool,
    user_id: UserId,
) -> Result<Vec<UserEmailView>, AppError> {
    let rows = sqlx::query_as::<_, UserEmailRow>(
        r#"
        SELECT id, email, verified_at, is_primary, created_at
        FROM user_emails
        WHERE user_id = $1
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub const SECONDARY_EMAIL_OTP_TTL_SECS: u64 = 600;

pub async fn add_user_email(
    db: &PgPool,
    redis: &RedisPool,
    user_id: UserId,
    raw_email: &str,
) -> Result<(UserEmailView, String), AppError> {
    let clean_email = raw_email.trim().to_lowercase();
    if !clean_email.contains('@') || clean_email.len() < 5 || clean_email.len() > 320 {
        return Err(AppError::Validation("Invalid email address".to_string()));
    }

    let row = sqlx::query_as::<_, UserEmailRow>(
        r#"
        INSERT INTO user_emails (user_id, email, verified_at, is_primary)
        VALUES ($1, $2, NULL, FALSE)
        ON CONFLICT (user_id, LOWER(email)) DO UPDATE SET updated_at = NOW()
        RETURNING id, email, verified_at, is_primary, created_at
        "#,
    )
    .bind(user_id)
    .bind(&clean_email)
    .fetch_one(db)
    .await?;

    let otp = generate_email_otp_code();
    crate::services::otp::store(
        redis,
        &email_otp_redis_key(user_id, &clean_email),
        &otp,
        SECONDARY_EMAIL_OTP_TTL_SECS,
    )
    .await?;

    Ok((row.into(), otp))
}

pub async fn verify_user_email(
    db: &PgPool,
    redis: &RedisPool,
    environment_id: EnvironmentId,
    user_id: UserId,
    raw_email: &str,
    code: &str,
) -> Result<UserEmailView, AppError> {
    let clean_email = raw_email.trim().to_lowercase();
    let key = email_otp_redis_key(user_id, &clean_email);
    if !crate::services::otp::consume(redis, &key, code).await? {
        return Err(AppError::Validation(
            "Invalid or expired email verification code".to_string(),
        ));
    }

    let mut tx = begin_tx(db).await?;
    let row = sqlx::query_as::<_, UserEmailRow>(
        r#"
        UPDATE user_emails
        SET verified_at = NOW(), updated_at = NOW()
        WHERE user_id = $1 AND LOWER(email) = $2
        RETURNING id, email, verified_at, is_primary, created_at
        "#,
    )
    .bind(user_id)
    .bind(&clean_email)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Email address not found on account".to_string()))?;

    let (user_version,): (i64,) = sqlx::query_as(
        r#"
        UPDATE users
        SET email_verified = TRUE, updated_at = NOW()
        WHERE id = $1
        RETURNING version
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::UserEmailVerified,
        Some(environment_id.into_inner()),
        AggregateType::User,
        user_id.into_inner(),
        user_version,
        serde_json::json!({
            "sub": user_id,
            "email": clean_email,
            "version": user_version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;
    commit_tx(tx).await?;
    Ok(row.into())
}

pub async fn set_primary_email(
    db: &PgPool,
    user_id: UserId,
    email_id: Uuid,
) -> Result<UserEmailView, AppError> {
    let mut tx = begin_tx(db).await?;
    let target = sqlx::query_as::<_, UserEmailRow>(
        r#"
        SELECT id, email, verified_at, is_primary, created_at
        FROM user_emails
        WHERE user_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(email_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Email address not found".to_string()))?;

    if target.verified_at.is_none() {
        return Err(AppError::Validation(
            "Only a verified email address can be promoted to primary".to_string(),
        ));
    }

    sqlx::query(
        r#"
        UPDATE user_emails
        SET is_primary = FALSE, updated_at = NOW()
        WHERE user_id = $1 AND is_primary = TRUE
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let promoted = sqlx::query_as::<_, UserEmailRow>(
        r#"
        UPDATE user_emails
        SET is_primary = TRUE, updated_at = NOW()
        WHERE user_id = $1 AND id = $2
        RETURNING id, email, verified_at, is_primary, created_at
        "#,
    )
    .bind(user_id)
    .bind(email_id)
    .fetch_one(&mut *tx)
    .await?;

    let (user_version,): (i64,) = sqlx::query_as(
        r#"
        UPDATE users
        SET email = $2, primary_email_id = $3, email_verified = TRUE, updated_at = NOW()
        WHERE id = $1
        RETURNING version
        "#,
    )
    .bind(user_id)
    .bind(&promoted.email)
    .bind(promoted.id)
    .fetch_one(&mut *tx)
    .await?;

    let joined_envs: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT environment_id, id
        FROM application_users
        WHERE user_id = $1 AND status <> 'revoked'
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await?;

    for (env_uuid, app_user_uuid) in joined_envs {
        let env_id = EnvironmentId::from(env_uuid);
        let envelope = DomainEventEnvelope::new(
            DomainEventType::UserUpdated,
            Some(env_uuid),
            AggregateType::User,
            user_id.into_inner(),
            user_version,
            serde_json::json!({
                "sub": user_id,
                "app_user_id": app_user_uuid,
                "email": promoted.email,
                "email_verified": true,
                "version": user_version
            }),
        );
        outbox::insert_outbox_event(&mut *tx, env_id, &envelope).await?;
    }

    commit_tx(tx).await?;
    Ok(promoted.into())
}

pub async fn remove_user_email(
    db: &PgPool,
    user_id: UserId,
    email_id: Uuid,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let target = sqlx::query_as::<_, UserEmailRow>(
        r#"
        SELECT id, email, verified_at, is_primary, created_at
        FROM user_emails
        WHERE user_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(email_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Email address not found".to_string()))?;

    let (verified_count,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM user_emails
        WHERE user_id = $1 AND verified_at IS NOT NULL
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    can_remove_email_guard(
        target.is_primary,
        target.verified_at.is_some(),
        verified_count,
    )?;

    sqlx::query(
        r#"
        DELETE FROM user_emails
        WHERE user_id = $1 AND id = $2
        "#,
    )
    .bind(user_id)
    .bind(email_id)
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    Ok(())
}

pub async fn list_oauth_accounts(
    db: &PgPool,
    user_id: UserId,
) -> Result<Vec<LinkedOAuthAccountView>, AppError> {
    let rows = sqlx::query_as::<_, OAuthAccountRow>(
        r#"
        SELECT id, provider, email, email_verified, created_at
        FROM oauth_accounts
        WHERE user_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn link_oauth_account(
    db: &PgPool,
    user_id: UserId,
    provider: &str,
    provider_user_id: &str,
    email: Option<&str>,
    email_verified: bool,
) -> Result<LinkedOAuthAccountView, AppError> {
    let clean_provider = provider.trim().to_lowercase();
    if !matches!(clean_provider.as_str(), "google" | "github") {
        return Err(AppError::Validation(format!(
            "Unsupported OAuth provider '{clean_provider}'"
        )));
    }
    let clean_provider_uid = provider_user_id.trim();
    if clean_provider_uid.is_empty() {
        return Err(AppError::Validation(
            "provider_user_id is required".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, OAuthAccountRow>(
        r#"
        INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, email_verified)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, provider, email, email_verified, created_at
        "#,
    )
    .bind(user_id)
    .bind(&clean_provider)
    .bind(clean_provider_uid)
    .bind(email.map(|e| e.trim().to_lowercase()))
    .bind(email_verified)
    .fetch_one(db)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::Validation("This OAuth provider account is already linked".to_string())
        }
        other => AppError::Database(other),
    })?;

    Ok(row.into())
}

pub async fn unlink_oauth_account(
    db: &PgPool,
    user_id: UserId,
    provider: &str,
) -> Result<(), AppError> {
    let clean_provider = provider.trim().to_lowercase();
    let mut tx = begin_tx(db).await?;

    let (has_password,): (bool,) = sqlx::query_as(
        r#"
        SELECT (password_hash IS NOT NULL AND password_hash <> '')
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let (linked_count,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM oauth_accounts
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let (exists_provider,): (bool,) = sqlx::query_as(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM oauth_accounts
            WHERE user_id = $1 AND LOWER(provider) = $2
        )
        "#,
    )
    .bind(user_id)
    .bind(&clean_provider)
    .fetch_one(&mut *tx)
    .await?;

    if !exists_provider {
        return Err(AppError::NotFound(format!(
            "OAuth provider '{clean_provider}' is not linked"
        )));
    }

    can_unlink_oauth_provider_guard(has_password, linked_count)?;

    sqlx::query(
        r#"
        DELETE FROM oauth_accounts
        WHERE user_id = $1 AND LOWER(provider) = $2
        "#,
    )
    .bind(user_id)
    .bind(&clean_provider)
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    Ok(())
}

pub async fn export_user_data(
    db: &PgPool,
    user_id: UserId,
) -> Result<IdentityDataExportResponse, AppError> {
    #[derive(FromRow)]
    struct UserRow {
        id: Uuid,
        email: String,
        display_name: String,
        avatar_url: Option<String>,
        email_verified: bool,
        status: String,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let user = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, display_name, avatar_url, email_verified, status, version, created_at, updated_at
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let emails = list_user_emails(db, user_id).await?;
    let oauth_accounts = list_oauth_accounts(db, user_id).await?;

    let memberships: Vec<(Uuid, Uuid, String, serde_json::Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT id, environment_id, status, public_metadata, joined_at
        FROM application_users
        WHERE user_id = $1
        ORDER BY joined_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    let orgs: Vec<(Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT o.id, o.slug, o.name, om.status, om.created_at
        FROM organization_members om
        JOIN application_users au
          ON au.environment_id = om.environment_id AND au.id = om.application_user_id
        JOIN organizations o
          ON o.environment_id = om.environment_id AND o.id = om.organization_id
        WHERE au.user_id = $1
        ORDER BY om.created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    let audit_rows: Vec<(Uuid, String, DateTime<Utc>, serde_json::Value)> = sqlx::query_as(
        r#"
        SELECT id, event, created_at, metadata
        FROM audit_logs
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(IdentityDataExportResponse {
        exported_at: Utc::now(),
        user: serde_json::json!({
            "sub": user.id,
            "email": user.email,
            "display_name": user.display_name,
            "avatar_url": user.avatar_url,
            "email_verified": user.email_verified,
            "status": user.status,
            "version": user.version,
            "created_at": user.created_at,
            "updated_at": user.updated_at
        }),
        emails,
        oauth_accounts,
        memberships: memberships
            .into_iter()
            .map(|(id, env_id, status, meta, joined)| {
                serde_json::json!({
                    "app_user_id": id,
                    "environment_id": env_id,
                    "status": status,
                    "public_metadata": meta,
                    "joined_at": joined
                })
            })
            .collect(),
        organizations: orgs
            .into_iter()
            .map(|(id, slug, name, status, joined)| {
                serde_json::json!({
                    "organization_id": id,
                    "slug": slug,
                    "name": name,
                    "status": status,
                    "joined_at": joined
                })
            })
            .collect(),
        recent_audit_logs: audit_rows
            .into_iter()
            .map(|(id, event, created_at, metadata)| {
                serde_json::json!({
                    "id": id,
                    "event": event,
                    "created_at": created_at,
                    "metadata": metadata
                })
            })
            .collect(),
    })
}

pub async fn delete_user_account(db: &PgPool, user_id: UserId) -> Result<usize, AppError> {
    let mut tx = begin_tx(db).await?;

    let (next_version,): (i64,) = sqlx::query_as(
        r#"
        UPDATE users
        SET status = 'deleted', email = 'deleted-' || id::text || '@erased.invalid', display_name = 'Deleted User', avatar_url = NULL, password_hash = NULL, email_verified = FALSE, primary_email_id = NULL, updated_at = NOW()
        WHERE id = $1
        RETURNING version
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::UserNotFound)?;

    sqlx::query(
        r#"
        UPDATE sso_sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW(), active_organization_id = NULL, updated_at = NOW()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    for statement in [
        "DELETE FROM user_emails WHERE user_id = $1",
        "DELETE FROM oauth_accounts WHERE user_id = $1",
        "DELETE FROM oidc_accounts WHERE user_id = $1",
        "DELETE FROM refresh_tokens WHERE user_id = $1",
        "DELETE FROM sessions WHERE user_id = $1",
        "DELETE FROM authentication_transactions WHERE sso_session_id IN (SELECT id FROM sso_sessions WHERE user_id = $1)",
        "UPDATE sso_sessions SET ip_address = NULL, user_agent = NULL WHERE user_id = $1",
        "UPDATE app_sessions SET ip_address = NULL, user_agent = NULL WHERE user_id = $1",
        "UPDATE audit_logs SET ip_address = NULL, user_agent = NULL, metadata = '{}'::jsonb WHERE user_id = $1 OR actor_id = $1",
        "UPDATE outbox_events SET payload = jsonb_set(payload, '{data}', jsonb_build_object('sub', $1::text, 'erased', true)) WHERE aggregate_id = $1 OR aggregate_id IN (SELECT id FROM application_users WHERE user_id = $1) OR payload->'data'->>'sub' = $1::text",
        "UPDATE webhook_deliveries SET response_body_excerpt = NULL, request_headers = '{}'::jsonb WHERE outbox_event_id IN (SELECT id FROM outbox_events WHERE aggregate_id = $1 OR aggregate_id IN (SELECT id FROM application_users WHERE user_id = $1) OR payload->'data'->>'sub' = $1::text)",
    ] {
        sqlx::query(statement).bind(user_id).execute(&mut *tx).await?;
    }

    let memberships: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        UPDATE application_users
        SET status = 'revoked', public_metadata = '{}'::jsonb, private_metadata = '{}'::jsonb, external_id = NULL, updated_at = NOW()
        WHERE user_id = $1
        RETURNING environment_id, id
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await?;

    let notified_envs = memberships.len();
    for (env_uuid, app_user_uuid) in memberships {
        let env_id = EnvironmentId::from(env_uuid);
        let envelope = DomainEventEnvelope::new(
            DomainEventType::UserDeleted,
            Some(env_uuid),
            AggregateType::User,
            user_id.into_inner(),
            next_version,
            serde_json::json!({
                "sub": user_id,
                "app_user_id": app_user_uuid,
                "version": next_version
            }),
        );
        outbox::insert_outbox_event(&mut *tx, env_id, &envelope).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO audit_logs (user_id, actor_type, actor_id, resource_type, resource_id, event, metadata)
        VALUES ($1, 'user', $1, 'user', $1, 'ACCOUNT_DELETED', $2)
        "#,
    )
    .bind(user_id)
    .bind(serde_json::json!({ "notified_environments": notified_envs }))
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    Ok(notified_envs)
}
