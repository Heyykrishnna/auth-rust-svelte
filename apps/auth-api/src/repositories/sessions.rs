use chrono::Utc;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::Session;

const SESSION_PREFIX: &str = "session:";
const BLACKLIST_PREFIX: &str = "blacklist:";

pub async fn store_redis_session(pool: &RedisPool, session: &Session) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let key = format!("{}{}", SESSION_PREFIX, session.id);
    let value = serde_json::to_string(session).map_err(|e| AppError::Internal(e.to_string()))?;
    let ttl = (session.expires_at - Utc::now()).num_seconds().max(0) as u64;

    let _: () = redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl)
        .arg(&value)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(())
}

pub async fn get_redis_session(
    pool: &RedisPool,
    session_id: Uuid,
) -> Result<Option<Session>, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let key = format!("{}{}", SESSION_PREFIX, session_id);

    let value: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    match value {
        Some(json) => {
            let session = serde_json::from_str::<Session>(&json)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

pub async fn delete_redis_session(pool: &RedisPool, session_id: Uuid) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let _: () = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(())
}

pub async fn blacklist_token(pool: &RedisPool, jti: &str, ttl_secs: u64) -> Result<(), AppError> {
    if ttl_secs == 0 {
        return Ok(());
    }
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let key = format!("{}{}", BLACKLIST_PREFIX, jti);
    let _: () = redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl_secs)
        .arg("1")
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(())
}

pub async fn is_token_blacklisted(pool: &RedisPool, jti: &str) -> Result<bool, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let key = format!("{}{}", BLACKLIST_PREFIX, jti);
    let exists: i64 = redis::cmd("EXISTS")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(exists > 0)
}

pub async fn create_pg_session(pool: &PgPool, session: &Session) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, refresh_token_hash, session_hash, user_agent, ip_address, created_at, expires_at, last_used_at, revoked_at)
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8, $9, $10)
        "#
    )
    .bind(session.id)
    .bind(session.user_id)
    .bind(&session.refresh_token_hash)
    .bind(&session.session_hash)
    .bind(&session.user_agent)
    .bind(&session.ip_address)
    .bind(session.created_at)
    .bind(session.expires_at)
    .bind(session.last_used_at)
    .bind(session.revoked_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_pg_session_by_id(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<Session>, AppError> {
    let session = sqlx::query_as::<_, Session>(
        r#"
        SELECT id, user_id, refresh_token_hash, session_hash, user_agent, host(ip_address) as ip_address, created_at, expires_at, last_used_at, revoked_at
        FROM sessions
        WHERE id = $1
        LIMIT 1
        "#
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    Ok(session)
}

pub async fn list_user_pg_sessions(pool: &PgPool, user_id: Uuid) -> Result<Vec<Session>, AppError> {
    let sessions = sqlx::query_as::<_, Session>(
        r#"
        SELECT id, user_id, refresh_token_hash, session_hash, user_agent, host(ip_address) as ip_address, created_at, expires_at, last_used_at, revoked_at
        FROM sessions
        WHERE user_id = $1 AND expires_at > NOW() AND revoked_at IS NULL
        ORDER BY last_used_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(sessions)
}

pub async fn revoke_pg_session(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE id = $1 AND revoked_at IS NULL")
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn revoke_all_user_pg_sessions(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_pg_session(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_all_user_pg_sessions(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}
