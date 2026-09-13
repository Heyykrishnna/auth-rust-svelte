use deadpool_redis::Pool as RedisPool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

const LOGIN_ATTEMPT_PREFIX: &str = "login_attempt:";
const VERIFY_CODE_PREFIX: &str = "verify_code:";
const PASSWORD_RESET_PREFIX: &str = "password_reset:";
const RATE_LIMIT_PREFIX: &str = "rate_limit:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationCodeData {
    pub user_id: Uuid,
    pub email: String,
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub async fn check_login_attempts(
    pool: &RedisPool,
    email: &str,
    max_attempts: u32,
) -> Result<Option<u32>, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", LOGIN_ATTEMPT_PREFIX, normalize_email(email));
    let val: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    if let Some(s) = val {
        if let Ok(attempts) = s.parse::<u32>() {
            if attempts >= max_attempts {
                return Err(AppError::TooManyRequests(format!(
                    "Account temporarily locked due to too many failed login attempts ({} attempts). Please try again in 15 minutes.",
                    attempts
                )));
            }
            return Ok(Some(attempts));
        }
    }

    Ok(None)
}

pub async fn record_failed_login(
    pool: &RedisPool,
    email: &str,
    lockout_secs: u64,
) -> Result<u32, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", LOGIN_ATTEMPT_PREFIX, normalize_email(email));
    let count: i64 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(lockout_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;
    }

    Ok(count as u32)
}

pub async fn clear_login_attempts(pool: &RedisPool, email: &str) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", LOGIN_ATTEMPT_PREFIX, normalize_email(email));
    let _: () = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(())
}

pub async fn store_verification_code(
    pool: &RedisPool,
    code: &str,
    data: &VerificationCodeData,
    ttl_secs: u64,
) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", VERIFY_CODE_PREFIX, code.trim());
    let value = serde_json::to_string(data).map_err(|e| AppError::Internal(e.to_string()))?;

    let _: () = redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl_secs)
        .arg(&value)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(())
}

pub async fn consume_verification_code(
    pool: &RedisPool,
    code: &str,
) -> Result<Option<VerificationCodeData>, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", VERIFY_CODE_PREFIX, code.trim());
    let val: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    match val {
        Some(json_str) => {
            let _: () = redis::cmd("DEL")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .map_err(|e| AppError::Redis(e.to_string()))?;

            let data = serde_json::from_str::<VerificationCodeData>(&json_str)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(data))
        }
        None => Ok(None),
    }
}

pub async fn store_password_reset_token(
    pool: &RedisPool,
    token: &str,
    user_id: Uuid,
    ttl_secs: u64,
) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", PASSWORD_RESET_PREFIX, token.trim());
    let value = user_id.to_string();

    let _: () = redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl_secs)
        .arg(&value)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    Ok(())
}

pub async fn consume_password_reset_token(
    pool: &RedisPool,
    token: &str,
) -> Result<Option<Uuid>, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", PASSWORD_RESET_PREFIX, token.trim());
    let val: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    match val {
        Some(user_id_str) => {
            let _: () = redis::cmd("DEL")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .map_err(|e| AppError::Redis(e.to_string()))?;

            let user_id = Uuid::parse_str(&user_id_str).map_err(|_| {
                AppError::InvalidToken("Malformed user ID in reset token".to_string())
            })?;
            Ok(Some(user_id))
        }
        None => Ok(None),
    }
}

pub async fn check_and_increment_rate_limit(
    pool: &RedisPool,
    identifier: &str,
    max_requests: u32,
    window_secs: u64,
) -> Result<bool, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    let key = format!("{}{}", RATE_LIMIT_PREFIX, identifier);
    let count: i64 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;

    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(window_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;
    }

    Ok(count <= max_requests as i64)
}
