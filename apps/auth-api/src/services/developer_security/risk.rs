use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::DeveloperId;
use crate::services::{account_security::detect_impossible_travel, email, tokens::hash_token};
use crate::AppState;

use super::{record_activity, redis_error, verification, SignInClient};

#[derive(Deserialize, Serialize, FromRow)]
pub struct NotificationPreferences {
    pub notify_new_sign_in: bool,
    pub notify_suspicious_sign_in: bool,
}

#[derive(Serialize, FromRow)]
pub struct Coverage {
    ip_reputation_configured: bool,
    location_data_configured: bool,
}

#[derive(FromRow)]
struct PriorSignIn {
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct AddressSignals {
    latitude: Option<f64>,
    longitude: Option<f64>,
    reputation: Option<String>,
}

pub async fn settings<'e>(
    db: impl sqlx::PgExecutor<'e>,
    id: DeveloperId,
) -> Result<NotificationPreferences, AppError> {
    Ok(sqlx::query_as(
        "SELECT COALESCE((SELECT notify_new_sign_in FROM developer_security_settings WHERE developer_id = $1), FALSE) AS notify_new_sign_in,
                COALESCE((SELECT notify_suspicious_sign_in FROM developer_security_settings WHERE developer_id = $1), TRUE) AS notify_suspicious_sign_in",
    ).bind(id).fetch_one(db).await?)
}

pub async fn set_preferences(
    state: &AppState,
    dev: &AuthenticatedDeveloper,
    preferences: NotificationPreferences,
) -> Result<(), AppError> {
    let mut tx = verification::lock_verified(&state.db, dev).await?;
    sqlx::query(
        "INSERT INTO developer_security_settings (developer_id, notify_new_sign_in, notify_suspicious_sign_in)
         VALUES ($1, $2, $3)
         ON CONFLICT (developer_id) DO UPDATE SET notify_new_sign_in = EXCLUDED.notify_new_sign_in,
             notify_suspicious_sign_in = EXCLUDED.notify_suspicious_sign_in",
    ).bind(dev.developer_id).bind(preferences.notify_new_sign_in).bind(preferences.notify_suspicious_sign_in)
        .execute(&mut *tx).await?;
    record_activity(
        &mut *tx,
        dev.developer_id,
        "DEVELOPER_NOTIFICATIONS_UPDATED",
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn coverage(db: &PgPool) -> Result<Coverage, AppError> {
    Ok(sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM account_ip_intelligence WHERE expires_at > NOW() AND reputation IS NOT NULL) AS ip_reputation_configured,
                EXISTS(SELECT 1 FROM account_ip_intelligence WHERE expires_at > NOW() AND latitude IS NOT NULL) AS location_data_configured",
    ).fetch_one(db).await?)
}

pub async fn network_limit(
    state: &AppState,
    ip: Option<&str>,
    email: &str,
    failed: bool,
) -> Result<bool, AppError> {
    let Some(ip) = ip
        .and_then(|ip| ip.parse::<IpAddr>().ok())
        .filter(|ip| !ip.is_loopback())
    else {
        return Ok(false);
    };
    let network = match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => std::net::Ipv6Addr::from(u128::from(ip) & (u128::MAX << 64)).to_string(),
    };
    let mut conn = state.redis.get().await.map_err(redis_error)?;
    let count: u32 = redis::Script::new(
        "local count = redis.call('SCARD', KEYS[1])
         if ARGV[1] == '1' and count < 12 then
             redis.call('SADD', KEYS[1], ARGV[2])
             redis.call('EXPIRE', KEYS[1], 600, 'NX')
             count = redis.call('SCARD', KEYS[1])
         end
         return count",
    )
    .key(format!(
        "developer:distinct-signin-failures:v1:{}",
        hash_token(&network)
    ))
    .arg(if failed { "1" } else { "0" })
    .arg(hash_token(email))
    .invoke_async(&mut conn)
    .await
    .map_err(redis_error)?;
    Ok(count >= 12)
}

pub async fn inspect_sign_in(
    tx: &mut Transaction<'_, Postgres>,
    id: DeveloperId,
    client: SignInClient<'_>,
    trusted: bool,
    recovery_code_used: bool,
) -> Result<Vec<&'static str>, AppError> {
    let mut flags = Vec::new();
    if recovery_code_used {
        flags.push("backup_code_used");
    }
    let failures: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM (SELECT 1 FROM audit_logs WHERE actor_type = 'developer' AND resource_type = 'developer'
         AND actor_id = $1 AND event = 'DEVELOPER_SIGN_IN_FAILED' AND created_at > NOW() - INTERVAL '15 minutes' LIMIT 3) AS recent",
    ).bind(id).fetch_one(&mut **tx).await?;
    if failures >= 3 {
        flags.push("recent_failed_sign_ins");
    }
    if let Some(ip) = client.ip {
        let previous = sqlx::query_as::<_, PriorSignIn>(
            "SELECT ip_address, user_agent, created_at FROM audit_logs WHERE actor_type = 'developer' AND resource_type = 'developer'
             AND actor_id = $1 AND event = 'DEVELOPER_SIGNED_IN' AND ip_address IS NOT NULL
             ORDER BY created_at DESC, id DESC LIMIT 1",
        ).bind(id).fetch_optional(&mut **tx).await?;
        let current = ip_signals(tx, ip).await?;
        if current.reputation.is_some() {
            flags.push("ip_reputation");
        }
        if let Some(previous) =
            previous.filter(|previous| previous.ip_address.as_deref() != Some(ip))
        {
            if !trusted && previous.user_agent.as_deref() != client.user_agent {
                flags.push("unfamiliar_browser");
            }
            if let Some(old_ip) = previous.ip_address {
                let prior = ip_signals(tx, &old_ip).await?;
                if detect_impossible_travel(
                    prior.latitude.zip(prior.longitude),
                    previous.created_at,
                    current.latitude.zip(current.longitude),
                    Utc::now(),
                )
                .is_some()
                {
                    flags.push("impossible_travel");
                }
            }
        }
    }
    if flags
        .iter()
        .filter(|flag| **flag != "backup_code_used")
        .count()
        >= 2
    {
        flags.push("possible_account_takeover");
    }
    Ok(flags)
}

async fn ip_signals(
    tx: &mut Transaction<'_, Postgres>,
    ip: &str,
) -> Result<AddressSignals, AppError> {
    Ok(sqlx::query_as(
        "SELECT location.latitude, location.longitude, reputation.reputation
         FROM (SELECT $1::inet AS ip) AS address
         LEFT JOIN LATERAL (
             SELECT latitude, longitude FROM account_ip_intelligence
             WHERE address.ip <<= network AND expires_at > NOW() AND latitude IS NOT NULL
             ORDER BY masklen(network) DESC LIMIT 1
         ) AS location ON TRUE
         LEFT JOIN LATERAL (
             SELECT reputation FROM account_ip_intelligence
             WHERE address.ip <<= network AND expires_at > NOW() AND reputation IS NOT NULL
             ORDER BY masklen(network) DESC LIMIT 1
         ) AS reputation ON TRUE",
    )
    .bind(ip)
    .fetch_one(&mut **tx)
    .await?)
}

pub fn notify(
    state: &AppState,
    id: DeveloperId,
    address: String,
    preferences: NotificationPreferences,
    suspicious: bool,
) {
    let requested = if suspicious {
        preferences.notify_suspicious_sign_in
    } else {
        preferences.notify_new_sign_in
    };
    if !requested || !email::smtp_configured(&state.config) {
        return;
    }
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = deliver(&state, id, &address, suspicious).await {
            tracing::warn!(
                kind = if matches!(error, AppError::Redis(_)) {
                    "queue"
                } else {
                    "transport"
                },
                "Developer sign-in notification delivery failed"
            );
        }
    });
}

async fn deliver(
    state: &AppState,
    id: DeveloperId,
    address: &str,
    suspicious: bool,
) -> Result<(), AppError> {
    let mut conn = state.redis.get().await.map_err(redis_error)?;
    let key = format!("developer:signin-email:v1:{id}:{suspicious}");
    let acquired = redis::cmd("SET")
        .arg(key)
        .arg("scheduled")
        .arg("NX")
        .arg("EX")
        .arg(300)
        .query_async::<_, Option<String>>(&mut conn)
        .await
        .map_err(redis_error)?;
    drop(conn);
    if acquired.is_some() {
        email::send_developer_signin_email(&state.config, address, suspicious).await?;
    }
    Ok(())
}
