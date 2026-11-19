use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::DeveloperId;
use crate::repositories::{redis_ephemeral, redis_keys};
use crate::services::password_hasher::verify_password_async;
use crate::AppState;

use super::{factors, record_activity};

pub async fn lock_account(
    tx: &mut Transaction<'_, Postgres>,
    id: DeveloperId,
) -> Result<Option<String>, AppError> {
    sqlx::query_scalar(
        "SELECT password_hash FROM developers WHERE id = $1 AND status = 'active' FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::InvalidCredentials)
}

async fn check_session(
    tx: &mut Transaction<'_, Postgres>,
    dev: &AuthenticatedDeveloper,
) -> Result<bool, AppError> {
    sqlx::query_scalar(
        "SELECT COALESCE(verified_until > NOW(), FALSE) FROM developer_sessions
         WHERE developer_id = $1 AND id = $2 AND revoked_at IS NULL AND expires_at > NOW()
         FOR UPDATE",
    )
    .bind(dev.developer_id)
    .bind(dev.session_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::SessionNotFound)
}

pub async fn lock_verified(
    db: &PgPool,
    dev: &AuthenticatedDeveloper,
) -> Result<Transaction<'static, Postgres>, AppError> {
    let mut tx = db.begin().await?;
    lock_account(&mut tx, dev.developer_id).await?;
    if !check_session(&mut tx, dev).await? {
        return Err(AppError::StepUpRequired);
    }
    Ok(tx)
}

pub async fn protect_existing_action(
    tx: &mut Transaction<'_, Postgres>,
    dev: &AuthenticatedDeveloper,
) -> Result<(), AppError> {
    let verified = check_session(tx, dev).await?;
    let has_factor: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM developer_security_settings WHERE developer_id = $1 AND totp_secret IS NOT NULL)
         OR EXISTS(SELECT 1 FROM developer_passkeys WHERE developer_id = $1)",
    )
    .bind(dev.developer_id)
    .fetch_one(&mut **tx)
    .await?;
    if has_factor && !verified {
        return Err(AppError::StepUpRequired);
    }
    Ok(())
}

pub async fn mark_verified(
    tx: &mut Transaction<'_, Postgres>,
    dev: &AuthenticatedDeveloper,
) -> Result<DateTime<Utc>, AppError> {
    sqlx::query_scalar(
        "UPDATE developer_sessions SET verified_until = LEAST(expires_at, NOW() + INTERVAL '5 minutes')
         WHERE developer_id = $1 AND id = $2 AND revoked_at IS NULL AND expires_at > NOW() RETURNING verified_until",
    )
    .bind(dev.developer_id)
    .bind(dev.session_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::SessionNotFound)
}

pub async fn proof_is_current(
    db: &PgPool,
    id: DeveloperId,
    session_id: Uuid,
) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM developer_sessions
         WHERE developer_id = $1 AND id = $2 AND revoked_at IS NULL AND expires_at > NOW() AND verified_until > NOW())",
    )
    .bind(id)
    .bind(session_id)
    .fetch_one(db)
    .await?)
}

pub fn attempt_key(id: DeveloperId) -> String {
    redis_keys::global_key("dev_step_up_attempt", &id.to_string())
}

pub async fn verify_password(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    password: String,
    code: Option<&str>,
) -> Result<DateTime<Utc>, AppError> {
    let key = attempt_key(dev.developer_id);
    redis_ephemeral::check_attempts(&state.redis, &key, state.config.login_max_attempts).await?;
    let hash = sqlx::query_scalar::<_, Option<String>>(
        "SELECT password_hash FROM developers WHERE id = $1 AND status = 'active'",
    )
    .bind(dev.developer_id)
    .fetch_optional(&state.db)
    .await?
    .flatten()
    .ok_or(AppError::InvalidCredentials)?;
    if !verify_password_async(password, hash.clone()).await? {
        redis_ephemeral::record_failed_attempt(
            &state.redis,
            &key,
            state.config.login_lockout_duration_secs,
        )
        .await?;
        return Err(AppError::InvalidCredentials);
    }
    let mut tx = state.db.begin().await?;
    if lock_account(&mut tx, dev.developer_id).await?.as_deref() != Some(&hash) {
        return Err(AppError::InvalidCredentials);
    }
    check_session(&mut tx, dev).await?;
    let factor =
        factors::verify_second_factor(&mut tx, &state.config, dev.developer_id, code).await;
    if let Err(error) = factor {
        tx.rollback().await?;
        if matches!(error, AppError::InvalidSecondFactor) {
            redis_ephemeral::record_failed_attempt(
                &state.redis,
                &key,
                state.config.login_lockout_duration_secs,
            )
            .await?;
        }
        return Err(error);
    }
    let expiry = mark_verified(&mut tx, dev).await?;
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_IDENTITY_VERIFIED",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(expiry)
}
