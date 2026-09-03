use anyhow::Result;
use deadpool_redis::{Config, Pool, Runtime};
use uuid::Uuid;

use crate::domain::session::Session;

/// Type alias for the Redis connection pool.
pub type RedisPool = Pool;

/// Create a Redis connection pool.
pub async fn connect(redis_url: &str) -> Result<RedisPool> {
    let cfg = Config::from_url(redis_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    // Verify connection
    let mut conn = pool.get().await?;
    redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await?;

    Ok(pool)
}

// ─── Session Operations ────────────────────────────────────────────────────────

const SESSION_PREFIX: &str = "session:";
const BLACKLIST_PREFIX: &str = "blacklist:";

/// Store a session in Redis with TTL.
pub async fn store_session(pool: &RedisPool, session: &Session) -> Result<()> {
    let mut conn = pool.get().await?;
    let key = format!("{}{}", SESSION_PREFIX, session.id);
    let value = serde_json::to_string(session)?;
    let ttl = (session.expires_at - chrono::Utc::now()).num_seconds().max(0) as u64;

    redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl)
        .arg(&value)
        .query_async::<()>(&mut conn)
        .await?;

    Ok(())
}

/// Retrieve a session from Redis.
pub async fn get_session(pool: &RedisPool, session_id: Uuid) -> Result<Option<Session>> {
    let mut conn = pool.get().await?;
    let key = format!("{}{}", SESSION_PREFIX, session_id);

    let value: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await?;

    match value {
        Some(json) => {
            let session = serde_json::from_str::<Session>(&json)?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

/// Delete a session from Redis.
pub async fn delete_session(pool: &RedisPool, session_id: Uuid) -> Result<()> {
    let mut conn = pool.get().await?;
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    redis::cmd("DEL").arg(&key).query_async::<()>(&mut conn).await?;
    Ok(())
}

/// Blacklist a JWT token by its JTI (for logout / revocation).
pub async fn blacklist_token(pool: &RedisPool, jti: &str, ttl_secs: u64) -> Result<()> {
    if ttl_secs == 0 {
        return Ok(()); // Token already expired, no need to blacklist
    }
    let mut conn = pool.get().await?;
    let key = format!("{}{}", BLACKLIST_PREFIX, jti);
    redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl_secs)
        .arg("1")
        .query_async::<()>(&mut conn)
        .await?;
    Ok(())
}

/// Check if a JWT token's JTI is blacklisted.
pub async fn is_token_blacklisted(pool: &RedisPool, jti: &str) -> Result<bool> {
    let mut conn = pool.get().await?;
    let key = format!("{}{}", BLACKLIST_PREFIX, jti);
    let exists: i64 = redis::cmd("EXISTS").arg(&key).query_async(&mut conn).await?;
    Ok(exists > 0)
}
