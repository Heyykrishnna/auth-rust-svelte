use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AuditEvent, AuditLog, NewAuditLog};
use crate::repositories::audit_logs as audit_repo;

pub async fn log_event(
    pool: &PgPool,
    user_id: Option<Uuid>,
    event: AuditEvent,
    ip_address: Option<String>,
    user_agent: Option<String>,
    metadata: serde_json::Value,
) -> Result<AuditLog, AppError> {
    let sanitized_metadata = sanitize_audit_metadata(metadata);

    let new_log = NewAuditLog {
        user_id,
        event,
        ip_address,
        user_agent,
        metadata: sanitized_metadata,
    };
    audit_repo::record_audit_log(pool, &new_log).await
}

pub async fn list_user_audit_events(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLog>, AppError> {
    audit_repo::list_user_audit_logs(pool, user_id, limit, offset).await
}

pub fn sanitize_audit_metadata(mut val: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(ref mut map) = val {
        for key in ["password", "password_hash", "token", "access_token", "refresh_token", "secret", "authorization"] {
            if map.contains_key(key) {
                map.insert(key.to_string(), serde_json::Value::String("[REDACTED]".to_string()));
            }
        }
    }
    val
}
