use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AuditEvent, AuditLog, NewAuditLog};

pub async fn record_audit_log(
    pool: &PgPool,
    new_log: &NewAuditLog,
) -> Result<AuditLog, AppError> {
    let log = sqlx::query_as::<_, AuditLog>(
        r#"
        INSERT INTO audit_logs (user_id, event, ip_address, user_agent, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, event, ip_address, user_agent, metadata, created_at
        "#
    )
    .bind(new_log.user_id)
    .bind(new_log.event)
    .bind(&new_log.ip_address)
    .bind(&new_log.user_agent)
    .bind(&new_log.metadata)
    .fetch_one(pool)
    .await?;

    Ok(log)
}

pub async fn record_audit_log_direct(
    pool: &PgPool,
    user_id: Option<Uuid>,
    event: AuditEvent,
    ip_address: Option<String>,
    user_agent: Option<String>,
    metadata: serde_json::Value,
) -> Result<AuditLog, AppError> {
    record_audit_log(
        pool,
        &NewAuditLog {
            user_id,
            event,
            ip_address,
            user_agent,
            metadata,
        },
    )
    .await
}

pub async fn list_user_audit_logs(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLog>, AppError> {
    let logs = sqlx::query_as::<_, AuditLog>(
        r#"
        SELECT id, user_id, event, ip_address, user_agent, metadata, created_at
        FROM audit_logs
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(logs)
}

pub async fn list_audit_logs(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLog>, AppError> {
    let logs = sqlx::query_as::<_, AuditLog>(
        r#"
        SELECT id, user_id, event, ip_address, user_agent, metadata, created_at
        FROM audit_logs
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(logs)
}
