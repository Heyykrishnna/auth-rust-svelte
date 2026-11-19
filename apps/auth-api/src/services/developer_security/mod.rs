pub mod devices;
pub mod factors;
pub mod passkeys;
pub mod recovery;
pub mod risk;
pub mod verification;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::DeveloperId;
use crate::AppState;

#[derive(Clone, Copy)]
pub struct SignInClient<'a> {
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub trusted_device: Option<&'a str>,
}

pub async fn record_activity<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    developer_id: DeveloperId,
    event: &str,
    client: Option<SignInClient<'_>>,
) -> Result<(), AppError> {
    record_event(executor, developer_id, event, client, &[]).await
}

pub async fn record_event<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    developer_id: DeveloperId,
    event: &str,
    client: Option<SignInClient<'_>>,
    flags: &[&str],
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_logs (actor_type, actor_id, resource_type, resource_id, event, ip_address, user_agent, metadata)
         VALUES ('developer', $1, 'developer', $1, $2, $3, $4, $5)",
    )
    .bind(developer_id)
    .bind(event)
    .bind(client.and_then(|info| info.ip))
    .bind(client.and_then(|info| info.user_agent))
    .bind(serde_json::json!({ "flags": flags }))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn create_session(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    developer_id: DeveloperId,
    expires_at: DateTime<Utc>,
    client: SignInClient<'_>,
    device_id: Option<Uuid>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO developer_sessions (id, developer_id, expires_at, ip_address, user_agent, trusted_device_id)
         VALUES ($1, $2, $3, CAST($4 AS inet), $5, $6)",
    )
    .bind(id)
    .bind(developer_id)
    .bind(expires_at)
    .bind(client.ip)
    .bind(client.user_agent)
    .bind(device_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn revoke_session(
    db: &PgPool,
    developer_id: DeveloperId,
    id: Uuid,
) -> Result<bool, AppError> {
    let mut tx = db.begin().await?;
    let changed = sqlx::query(
        "UPDATE developer_sessions SET revoked_at = NOW()
         WHERE id = $1 AND developer_id = $2 AND revoked_at IS NULL AND expires_at > NOW()",
    )
    .bind(id)
    .bind(developer_id)
    .execute(&mut *tx)
    .await?;
    if changed.rows_affected() == 1 {
        record_activity(&mut *tx, developer_id, "DEVELOPER_SESSION_REVOKED", None).await?;
    }
    tx.commit().await?;
    Ok(changed.rows_affected() == 1)
}

pub async fn revoke_other_sessions(
    tx: &mut Transaction<'_, Postgres>,
    developer_id: DeveloperId,
    current_session: Uuid,
) -> Result<(), AppError> {
    revoke_account_sessions(tx, developer_id, RevokeSessions::Except(current_session)).await
}

pub enum RevokeSessions {
    All,
    Except(Uuid),
}

pub async fn revoke_account_sessions(
    tx: &mut Transaction<'_, Postgres>,
    developer_id: DeveloperId,
    selection: RevokeSessions,
) -> Result<(), AppError> {
    let except = match selection {
        RevokeSessions::All => None,
        RevokeSessions::Except(id) => Some(id),
    };
    sqlx::query(
        "UPDATE developer_sessions SET revoked_at = NOW()
         WHERE developer_id = $1 AND id IS DISTINCT FROM $2 AND revoked_at IS NULL AND expires_at > NOW()",
    )
    .bind(developer_id)
    .bind(except)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn revoke_other_session(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    id: Uuid,
) -> Result<(), AppError> {
    if id == dev.session_id {
        return Err(AppError::Validation(
            "Use sign out to end your current session".to_string(),
        ));
    }
    let mut tx = state.db.begin().await?;
    verification::lock_account(&mut tx, dev.developer_id).await?;
    verification::protect_existing_action(&mut tx, dev).await?;
    let changed = sqlx::query(
        "UPDATE developer_sessions SET revoked_at = NOW() WHERE id = $1 AND developer_id = $2
         AND revoked_at IS NULL AND expires_at > NOW()",
    )
    .bind(id)
    .bind(dev.developer_id)
    .execute(&mut *tx)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::NotFound("Active session not found".to_string()));
    }
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_SESSION_REVOKED",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, FromRow)]
struct ActiveSession {
    id: Uuid,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
    current: bool,
    trusted_device: Option<String>,
}

#[derive(Serialize, FromRow)]
struct Activity {
    id: Uuid,
    event: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
    flags: serde_json::Value,
}

#[derive(Serialize)]
pub struct Overview {
    sessions: Vec<ActiveSession>,
    recent_activity: Vec<Activity>,
    brute_force_protection: bool,
    credential_stuffing_protection: bool,
    totp_enabled: bool,
    backup_codes_remaining: i64,
    verified_until: Option<DateTime<Utc>>,
    passkeys_configured: bool,
    passkeys: Vec<passkeys::PasskeyInfo>,
    trusted_devices: Vec<devices::Device>,
    notifications: risk::NotificationPreferences,
    delivery: recovery::DeliveryStatus,
    risk_coverage: risk::Coverage,
}

pub async fn overview(
    state: &AppState,
    developer: &AuthenticatedDeveloper,
) -> Result<Overview, AppError> {
    let sessions = sqlx::query_as(
        "SELECT s.id, host(s.ip_address) AS ip_address, s.user_agent, s.created_at, s.id = $2 AS current,
                d.label AS trusted_device
         FROM developer_sessions s LEFT JOIN developer_trusted_devices d ON d.id = s.trusted_device_id
           AND d.developer_id = s.developer_id AND d.expires_at > NOW()
         WHERE s.developer_id = $1 AND s.revoked_at IS NULL AND s.expires_at > NOW()
         ORDER BY current DESC, s.created_at DESC, s.id DESC LIMIT 50",
    )
    .bind(developer.developer_id)
    .bind(developer.session_id)
    .fetch_all(&state.db);
    let history = sqlx::query_as(
        "SELECT id, event, ip_address, user_agent, created_at, COALESCE(metadata->'flags', '[]'::jsonb) AS flags
         FROM audit_logs WHERE actor_type = 'developer' AND resource_type = 'developer' AND actor_id = $1
         AND event IN (
             'DEVELOPER_REGISTERED', 'DEVELOPER_SIGNED_IN', 'DEVELOPER_SIGN_IN_FAILED',
             'DEVELOPER_SESSION_REVOKED', 'DEVELOPER_PASSWORD_CHANGED', 'DEVELOPER_PASSWORD_RESET',
             'DEVELOPER_MFA_ENABLED', 'DEVELOPER_MFA_DISABLED', 'DEVELOPER_BACKUP_CODES_REPLACED',
             'DEVELOPER_BACKUP_CODE_USED', 'DEVELOPER_PASSKEY_ADDED', 'DEVELOPER_PASSKEY_REMOVED',
             'DEVELOPER_DEVICE_TRUSTED', 'DEVELOPER_DEVICE_FORGOTTEN',
             'DEVELOPER_NOTIFICATIONS_UPDATED', 'DEVELOPER_IDENTITY_VERIFIED',
             'DEVELOPER_SUSPICIOUS_SIGN_IN', 'DEVELOPER_CREDENTIAL_STUFFING'
         )
         ORDER BY created_at DESC, id DESC LIMIT 50",
    )
    .bind(developer.developer_id)
    .fetch_all(&state.db);
    let factors = sqlx::query_as::<_, (bool, i64, Option<DateTime<Utc>>)>(
        "SELECT EXISTS(SELECT 1 FROM developer_security_settings WHERE developer_id = $1 AND totp_secret IS NOT NULL),
                (SELECT COUNT(*) FROM developer_backup_codes WHERE developer_id = $1),
                (SELECT verified_until FROM developer_sessions WHERE developer_id = $1 AND id = $2 AND verified_until > NOW())",
    ).bind(developer.developer_id).bind(developer.session_id).fetch_one(&state.db);
    let (
        sessions,
        recent_activity,
        (totp_enabled, backup_codes_remaining, verified_until),
        passkeys,
        trusted_devices,
        notifications,
        risk_coverage,
    ) = tokio::try_join!(
        async { sessions.await.map_err(AppError::from) },
        async { history.await.map_err(AppError::from) },
        async { factors.await.map_err(AppError::from) },
        passkeys::list(state, developer.developer_id),
        devices::list(state, developer),
        risk::settings(&state.db, developer.developer_id),
        risk::coverage(&state.db),
    )?;
    Ok(Overview {
        sessions,
        recent_activity,
        totp_enabled,
        backup_codes_remaining,
        verified_until,
        passkeys,
        trusted_devices,
        notifications,
        risk_coverage,
        brute_force_protection: state.config.login_max_attempts > 0
            && state.config.login_lockout_duration_secs > 0,
        credential_stuffing_protection: true,
        passkeys_configured: passkeys::available(state),
        delivery: recovery::delivery_status(state),
    })
}

pub(super) fn console_origin(config: &AppConfig) -> Result<url::Url, AppError> {
    let valid = config
        .cors_origins
        .first()
        .and_then(|origin| url::Url::parse(origin).ok())
        .filter(|url| {
            (url.scheme() == "https"
                || (!config.cookie_secure
                    && url.scheme() == "http"
                    && url.host_str() == Some("localhost")))
                && url.host().is_some()
                && url.path() == "/"
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        });
    valid.ok_or_else(|| {
        AppError::Validation("A valid HTTPS Developer Console origin is required".to_string())
    })
}

pub(super) fn validate_label(s: &str) -> Result<&str, AppError> {
    let s = s.trim();
    if !(1..=80).contains(&s.chars().count()) || s.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Enter a name from 1 to 80 characters".to_string(),
        ));
    }
    Ok(s)
}

pub(super) fn redis_error(error: impl std::fmt::Display) -> AppError {
    AppError::Redis(error.to_string())
}
