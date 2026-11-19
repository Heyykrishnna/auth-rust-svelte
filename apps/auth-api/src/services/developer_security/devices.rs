use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::DeveloperId;
use crate::services::tokens::{generate_opaque_refresh_token, hash_token};
use crate::AppState;

use super::{record_activity, validate_label, verification};

pub const COOKIE_NAME: &str = "signet_dev_device";

#[derive(Serialize, FromRow)]
pub struct Device {
    id: Uuid,
    label: String,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    current: bool,
}

pub async fn list(state: &AppState, dev: &AuthenticatedDeveloper) -> Result<Vec<Device>, AppError> {
    Ok(sqlx::query_as(
        "SELECT d.id, d.label, d.created_at, d.last_seen_at, d.expires_at,
           EXISTS(SELECT 1 FROM developer_sessions WHERE id = $2 AND trusted_device_id = d.id) AS current
         FROM developer_trusted_devices d WHERE d.developer_id = $1 AND d.expires_at > NOW()
         ORDER BY current DESC, d.last_seen_at DESC, d.id LIMIT 20",
    ).bind(dev.developer_id).bind(dev.session_id).fetch_all(&state.db).await?)
}

pub async fn trust(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    label: &str,
) -> Result<Cookie<'static>, AppError> {
    let label = validate_label(label)?;
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    let current: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM developer_sessions s JOIN developer_trusted_devices d ON d.id = s.trusted_device_id
          WHERE s.id = $1 AND d.developer_id = $2 AND d.expires_at > NOW())",
    ).bind(dev.session_id).bind(dev.developer_id).fetch_one(&mut *tx).await?;
    if current {
        return Err(AppError::Validation(
            "This browser is already trusted".to_string(),
        ));
    }
    let token = generate_opaque_refresh_token();
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO developer_trusted_devices (developer_id, token_hash, label, expires_at)
         SELECT $1, $2, $3, NOW() + INTERVAL '30 days'
         WHERE (SELECT COUNT(*) FROM developer_trusted_devices WHERE developer_id = $1 AND expires_at > NOW()) < 20
         RETURNING id",
    ).bind(dev.developer_id).bind(hash_token(&token)).bind(label).fetch_optional(&mut *tx).await?
      .ok_or_else(|| AppError::Validation("Remove a trusted browser before adding another".to_string()))?;
    sqlx::query("UPDATE developer_sessions SET trusted_device_id = $2 WHERE id = $1")
        .bind(dev.session_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    record_activity(&mut *tx, dev.developer_id, "DEVELOPER_DEVICE_TRUSTED", None).await?;
    tx.commit().await?;
    Ok(cookie(&state.config, token, time::Duration::days(30)))
}

pub async fn forget(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    id: Uuid,
) -> Result<bool, AppError> {
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    let current: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM developer_sessions WHERE id = $1 AND trusted_device_id = $2)",
    )
    .bind(dev.session_id)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE developer_sessions SET revoked_at = NOW() WHERE developer_id = $1
         AND trusted_device_id = $2 AND id <> $3 AND revoked_at IS NULL AND expires_at > NOW()",
    )
    .bind(dev.developer_id)
    .bind(id)
    .bind(dev.session_id)
    .execute(&mut *tx)
    .await?;
    let deleted =
        sqlx::query("DELETE FROM developer_trusted_devices WHERE developer_id = $1 AND id = $2")
            .bind(dev.developer_id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    if deleted.rows_affected() != 1 {
        return Err(AppError::NotFound("Trusted browser not found".to_string()));
    }
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_DEVICE_FORGOTTEN",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(current)
}

pub fn clear_cookie(config: &AppConfig) -> Cookie<'static> {
    cookie(config, String::new(), time::Duration::ZERO)
}

fn cookie(config: &AppConfig, token: String, age: time::Duration) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, token))
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true)
        .secure(config.cookie_secure)
        .max_age(age)
        .build()
}

pub async fn observe_sign_in(
    tx: &mut Transaction<'_, Postgres>,
    id: DeveloperId,
    token: Option<&str>,
) -> Result<Option<Uuid>, AppError> {
    let Some(token) = token.filter(|t| !t.is_empty() && t.len() <= 128) else {
        return Ok(None);
    };
    Ok(sqlx::query_scalar(
        "UPDATE developer_trusted_devices SET last_seen_at = NOW()
         WHERE developer_id = $1 AND token_hash = $2 AND expires_at > NOW() RETURNING id",
    )
    .bind(id)
    .bind(hash_token(token))
    .fetch_optional(&mut **tx)
    .await?)
}
