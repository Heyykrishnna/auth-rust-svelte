use sqlx::PgExecutor;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{DomainEventEnvelope, EnvironmentId};

pub async fn insert_outbox_event<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    envelope: &DomainEventEnvelope,
) -> Result<Uuid, AppError>
where
    E: PgExecutor<'e>,
{
    let event_type = serde_json::to_value(&envelope.event_type)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .ok_or_else(|| AppError::Internal("Failed to serialize domain event type".to_string()))?;

    let aggregate_type = serde_json::to_value(&envelope.aggregate_type)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .ok_or_else(|| AppError::Internal("Failed to serialize aggregate type".to_string()))?;

    let payload = serde_json::to_value(envelope).map_err(|e| AppError::Internal(e.to_string()))?;

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO outbox_events (
            id, environment_id, event_type, aggregate_type, aggregate_id, aggregate_version, payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(envelope.event_id)
    .bind(environment_id)
    .bind(event_type)
    .bind(aggregate_type)
    .bind(envelope.aggregate_id)
    .bind(envelope.aggregate_version)
    .bind(payload)
    .fetch_one(executor)
    .await?;

    Ok(id)
}
