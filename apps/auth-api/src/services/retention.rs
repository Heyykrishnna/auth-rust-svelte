use crate::errors::AppError;
use sqlx::PgPool;

pub async fn purge_expired_records(pool: &PgPool, retention_days: u32) -> Result<u64, AppError> {
    if !(30..=3650).contains(&retention_days) {
        return Err(AppError::Validation(
            "Retention must be between 30 and 3650 days".to_string(),
        ));
    }
    let days = i32::try_from(retention_days)
        .map_err(|_| AppError::Validation("Invalid retention period".to_string()))?;
    let mut transaction = pool.begin().await?;
    let mut removed = 0;
    for statement in [
        "DELETE FROM audit_logs WHERE id IN (SELECT id FROM audit_logs WHERE created_at < NOW() - make_interval(days => $1::int) ORDER BY created_at LIMIT 500 FOR UPDATE SKIP LOCKED)",
        "DELETE FROM webhook_deliveries WHERE id IN (SELECT id FROM webhook_deliveries WHERE status IN ('succeeded', 'failed', 'dead_letter') AND updated_at < NOW() - make_interval(days => $1::int) ORDER BY updated_at LIMIT 500 FOR UPDATE SKIP LOCKED)",
        "DELETE FROM outbox_events WHERE id IN (SELECT e.id FROM outbox_events e WHERE e.status IN ('published', 'dead_letter') AND e.updated_at < NOW() - make_interval(days => $1::int) AND NOT EXISTS (SELECT 1 FROM webhook_deliveries d WHERE d.outbox_event_id = e.id AND d.status IN ('pending', 'processing', 'retrying')) ORDER BY e.updated_at LIMIT 500 FOR UPDATE SKIP LOCKED)",
        "DELETE FROM authentication_transactions WHERE id IN (SELECT id FROM authentication_transactions WHERE expires_at < NOW() - INTERVAL '1 day' ORDER BY expires_at LIMIT 500 FOR UPDATE SKIP LOCKED)",
    ] {
        let result = if statement.contains("$1") {
            sqlx::query(statement).bind(days).execute(&mut *transaction).await?
        } else {
            sqlx::query(statement).execute(&mut *transaction).await?
        };
        removed += result.rows_affected();
    }
    transaction.commit().await?;
    metrics::counter!("auth_retention_records_purged_total").increment(removed);
    Ok(removed)
}
