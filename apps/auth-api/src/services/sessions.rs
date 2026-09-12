use chrono::Utc;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{RefreshToken, Session, SessionResponse};
use crate::repositories::refresh_tokens as refresh_repo;
use crate::repositories::sessions as session_repo;
use crate::services::tokens::hash_token;

pub async fn create_session(
    db: &PgPool,
    redis: &RedisPool,
    user_id: Uuid,
    refresh_token: &str,
    duration: Duration,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<Session, AppError> {
    let token_hash = hash_token(refresh_token);
    let expires_at = Utc::now()
        + chrono::Duration::from_std(duration).map_err(|e| AppError::Internal(e.to_string()))?;

    let session = Session::new(
        user_id,
        token_hash.clone(),
        expires_at,
        user_agent,
        ip_address,
    );

    session_repo::store_redis_session(redis, &session).await?;
    let _ = session_repo::create_pg_session(db, &session).await;

    let refresh_token_record = RefreshToken::new(
        user_id,
        Some(session.id),
        token_hash,
        session.id,
        expires_at,
    );
    let _ = refresh_repo::create_refresh_token(db, &refresh_token_record).await;

    Ok(session)
}

pub async fn get_session(
    db: &PgPool,
    redis: &RedisPool,
    session_id: Uuid,
) -> Result<Option<Session>, AppError> {
    if let Ok(Some(session)) = session_repo::get_redis_session(redis, session_id).await {
        if session.is_active() {
            return Ok(Some(session));
        } else {
            let _ = session_repo::delete_redis_session(redis, session_id).await;
            return Ok(None);
        }
    }

    match session_repo::find_pg_session_by_id(db, session_id).await? {
        Some(session) if session.is_active() => Ok(Some(session)),
        _ => Ok(None),
    }
}

pub async fn invalidate_session(
    db: &PgPool,
    redis: &RedisPool,
    session_id: Uuid,
) -> Result<(), AppError> {
    let _ = session_repo::delete_redis_session(redis, session_id).await;
    let _ = session_repo::revoke_pg_session(db, session_id).await;
    let _ = session_repo::delete_pg_session(db, session_id).await;
    let _ = refresh_repo::revoke_family(db, session_id).await;
    Ok(())
}

pub async fn list_user_sessions(
    db: &PgPool,
    user_id: Uuid,
    current_session_id: Option<Uuid>,
) -> Result<Vec<SessionResponse>, AppError> {
    let sessions = session_repo::list_user_pg_sessions(db, user_id).await?;
    let response = sessions
        .into_iter()
        .map(|s| {
            let is_current = current_session_id.map(|cid| cid == s.id).unwrap_or(false);
            SessionResponse {
                id: s.id,
                user_agent: s.user_agent,
                ip_address: s.ip_address,
                created_at: s.created_at,
                last_used_at: s.last_used_at,
                is_current,
            }
        })
        .collect();

    Ok(response)
}

pub async fn revoke_all_user_sessions(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let _ = refresh_repo::revoke_all_user_tokens(db, user_id).await;
    let _ = session_repo::revoke_all_user_pg_sessions(db, user_id).await;
    session_repo::delete_all_user_pg_sessions(db, user_id).await
}
