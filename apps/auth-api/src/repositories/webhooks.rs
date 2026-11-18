use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor, PgPool};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::EnvironmentId;
use crate::repositories::db::PgTx;

#[derive(Debug, Clone, FromRow)]
pub struct WebhookEndpointRecord {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub url: String,
    pub description: String,
    pub subscribed_events: Vec<String>,
    pub signing_secret_encrypted: Vec<u8>,
    pub kek_version: String,
    pub previous_signing_secret_encrypted: Option<Vec<u8>>,
    pub previous_secret_expires_at: Option<DateTime<Utc>>,
    pub status: String,
    pub consecutive_failures: i32,
    pub paused_reason: Option<String>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookDeliveryRecord {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub webhook_endpoint_id: Uuid,
    pub outbox_event_id: Uuid,
    pub attempt_number: i32,
    pub request_headers: serde_json::Value,
    pub response_status: Option<i32>,
    pub response_body_excerpt: Option<String>,
    pub latency_ms: Option<i32>,
    pub status: String,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OutboxEventRecord {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub aggregate_version: i64,
    pub payload: serde_json::Value,
    pub status: String,
    pub available_at: DateTime<Utc>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DueDeliveryJob {
    pub delivery_id: Uuid,
    pub environment_id: EnvironmentId,
    pub environment_kind: String,
    pub webhook_endpoint_id: Uuid,
    pub outbox_event_id: Uuid,
    pub attempt_number: i32,
    pub endpoint_url: String,
    pub endpoint_status: String,
    pub signing_secret_encrypted: Vec<u8>,
    pub kek_version: String,
    pub previous_signing_secret_encrypted: Option<Vec<u8>>,
    pub previous_secret_expires_at: Option<DateTime<Utc>>,
    pub event_type: String,
    pub event_payload: serde_json::Value,
    pub event_traceparent: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_endpoint<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    url: &str,
    description: &str,
    subscribed_events: &[String],
    signing_secret_encrypted: &[u8],
    kek_version: &str,
) -> Result<WebhookEndpointRecord, AppError>
where
    E: PgExecutor<'e>,
{
    let rec = sqlx::query_as::<_, WebhookEndpointRecord>(
        r#"
        INSERT INTO webhook_endpoints (
            environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'active')
        RETURNING id, environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version,
                  previous_signing_secret_encrypted, previous_secret_expires_at, status, consecutive_failures,
                  paused_reason, last_failure_at, last_success_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(url)
    .bind(description)
    .bind(subscribed_events)
    .bind(signing_secret_encrypted)
    .bind(kek_version)
    .fetch_one(executor)
    .await?;

    Ok(rec)
}

pub async fn list_endpoints_for_environment(
    db: &PgPool,
    environment_id: EnvironmentId,
) -> Result<Vec<WebhookEndpointRecord>, AppError> {
    let rows = sqlx::query_as::<_, WebhookEndpointRecord>(
        r#"
        SELECT id, environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version,
               previous_signing_secret_encrypted, previous_secret_expires_at, status, consecutive_failures,
               paused_reason, last_failure_at, last_success_at, created_at, updated_at
        FROM webhook_endpoints
        WHERE environment_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(environment_id)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

pub async fn find_endpoint_by_id<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
) -> Result<Option<WebhookEndpointRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let row = sqlx::query_as::<_, WebhookEndpointRecord>(
        r#"
        SELECT id, environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version,
               previous_signing_secret_encrypted, previous_secret_expires_at, status, consecutive_failures,
               paused_reason, last_failure_at, last_success_at, created_at, updated_at
        FROM webhook_endpoints
        WHERE environment_id = $1 AND id = $2
        "#,
    )
    .bind(environment_id)
    .bind(endpoint_id)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_endpoint<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
    url: Option<&str>,
    description: Option<&str>,
    subscribed_events: Option<&[String]>,
    status: Option<&str>,
) -> Result<Option<WebhookEndpointRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let reset_failures = matches!(status, Some("active"));
    let row = sqlx::query_as::<_, WebhookEndpointRecord>(
        r#"
        UPDATE webhook_endpoints
        SET
            url = COALESCE($3, url),
            description = COALESCE($4, description),
            subscribed_events = COALESCE($5, subscribed_events),
            status = COALESCE($6, status),
            consecutive_failures = CASE WHEN $7 THEN 0 ELSE consecutive_failures END,
            paused_reason = CASE WHEN $7 THEN NULL ELSE paused_reason END,
            updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version,
                  previous_signing_secret_encrypted, previous_secret_expires_at, status, consecutive_failures,
                  paused_reason, last_failure_at, last_success_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(endpoint_id)
    .bind(url)
    .bind(description)
    .bind(subscribed_events)
    .bind(status)
    .bind(reset_failures)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

pub async fn rotate_endpoint_secret<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
    new_secret_encrypted: &[u8],
    new_kek_version: &str,
    grace_expires_at: DateTime<Utc>,
) -> Result<Option<WebhookEndpointRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let row = sqlx::query_as::<_, WebhookEndpointRecord>(
        r#"
        UPDATE webhook_endpoints
        SET
            previous_signing_secret_encrypted = signing_secret_encrypted,
            previous_secret_expires_at = $5,
            signing_secret_encrypted = $3,
            kek_version = $4,
            updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version,
                  previous_signing_secret_encrypted, previous_secret_expires_at, status, consecutive_failures,
                  paused_reason, last_failure_at, last_success_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(endpoint_id)
    .bind(new_secret_encrypted)
    .bind(new_kek_version)
    .bind(grace_expires_at)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

pub async fn delete_endpoint<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
) -> Result<bool, AppError>
where
    E: PgExecutor<'e>,
{
    let res = sqlx::query("DELETE FROM webhook_endpoints WHERE environment_id = $1 AND id = $2")
        .bind(environment_id)
        .bind(endpoint_id)
        .execute(executor)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn claim_pending_outbox_events_in_tx(
    tx: &mut PgTx<'_>,
    batch_size: i64,
) -> Result<Vec<OutboxEventRecord>, AppError> {
    let rows = sqlx::query_as::<_, OutboxEventRecord>(
        r#"
        WITH claimed AS (
            SELECT id
            FROM outbox_events
            WHERE status = 'pending' AND available_at <= NOW()
            ORDER BY created_at ASC, id ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE outbox_events oe
        SET status = 'processing', attempts = oe.attempts + 1, updated_at = NOW()
        FROM claimed
        WHERE oe.id = claimed.id
        RETURNING oe.id, oe.environment_id, oe.event_type, oe.aggregate_type, oe.aggregate_id,
                  oe.aggregate_version, oe.payload, oe.status, oe.available_at, oe.attempts,
                  oe.created_at, oe.updated_at
        "#,
    )
    .bind(batch_size)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows)
}

pub async fn find_active_matching_endpoints_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    event_type: &str,
) -> Result<Vec<WebhookEndpointRecord>, AppError> {
    let rows = sqlx::query_as::<_, WebhookEndpointRecord>(
        r#"
        SELECT id, environment_id, url, description, subscribed_events, signing_secret_encrypted, kek_version,
               previous_signing_secret_encrypted, previous_secret_expires_at, status, consecutive_failures,
               paused_reason, last_failure_at, last_success_at, created_at, updated_at
        FROM webhook_endpoints
        WHERE environment_id = $1
          AND status = 'active'
          AND (
              cardinality(subscribed_events) = 0
              OR '*' = ANY(subscribed_events)
              OR $2 = ANY(subscribed_events)
          )
        "#,
    )
    .bind(environment_id)
    .bind(event_type)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows)
}

pub async fn enqueue_delivery_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    webhook_endpoint_id: Uuid,
    outbox_event_id: Uuid,
    attempt_number: i32,
) -> Result<Option<Uuid>, AppError> {
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO webhook_deliveries (
            environment_id, webhook_endpoint_id, outbox_event_id, attempt_number, status, next_retry_at
        )
        VALUES ($1, $2, $3, $4, 'pending', NOW())
        ON CONFLICT (webhook_endpoint_id, outbox_event_id, attempt_number) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(environment_id)
    .bind(webhook_endpoint_id)
    .bind(outbox_event_id)
    .bind(attempt_number)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(id)
}

pub async fn mark_outbox_event_published_in_tx(
    tx: &mut PgTx<'_>,
    outbox_event_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("UPDATE outbox_events SET status = 'published', updated_at = NOW() WHERE id = $1")
        .bind(outbox_event_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn claim_due_deliveries_in_tx(
    tx: &mut PgTx<'_>,
    batch_size: i64,
) -> Result<Vec<DueDeliveryJob>, AppError> {
    let jobs = sqlx::query_as::<_, DueDeliveryJob>(
        r#"
        SELECT
            wd.id AS delivery_id,
            wd.environment_id,
            ae.kind AS environment_kind,
            wd.webhook_endpoint_id,
            wd.outbox_event_id,
            wd.attempt_number,
            we.url AS endpoint_url,
            we.status AS endpoint_status,
            we.signing_secret_encrypted,
            we.kek_version,
            we.previous_signing_secret_encrypted,
            we.previous_secret_expires_at,
            oe.event_type,
            oe.payload AS event_payload,
            oe.traceparent AS event_traceparent
        FROM webhook_deliveries wd
        JOIN webhook_endpoints we
          ON we.environment_id = wd.environment_id AND we.id = wd.webhook_endpoint_id
        JOIN outbox_events oe
          ON oe.environment_id = wd.environment_id AND oe.id = wd.outbox_event_id
        JOIN application_environments ae
          ON ae.id = wd.environment_id
        WHERE wd.status IN ('pending', 'retrying')
          AND (wd.next_retry_at IS NULL OR wd.next_retry_at <= NOW())
        ORDER BY wd.created_at ASC, wd.id ASC
        LIMIT $1
        FOR UPDATE OF wd SKIP LOCKED
        "#,
    )
    .bind(batch_size)
    .fetch_all(&mut **tx)
    .await?;

    Ok(jobs)
}

#[allow(clippy::too_many_arguments)]
pub async fn mark_delivery_succeeded_in_tx(
    tx: &mut PgTx<'_>,
    delivery_id: Uuid,
    endpoint_id: Uuid,
    request_headers: &serde_json::Value,
    response_status: i32,
    response_body_excerpt: &str,
    latency_ms: i32,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE webhook_deliveries
        SET
            status = 'succeeded',
            request_headers = $2,
            response_status = $3,
            response_body_excerpt = $4,
            latency_ms = $5,
            next_retry_at = NULL,
            delivered_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(request_headers)
    .bind(response_status)
    .bind(response_body_excerpt)
    .bind(latency_ms)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE webhook_endpoints
        SET consecutive_failures = 0, last_success_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(endpoint_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn mark_delivery_failed_and_schedule_next_in_tx(
    tx: &mut PgTx<'_>,
    job: &DueDeliveryJob,
    request_headers: &serde_json::Value,
    response_status: Option<i32>,
    response_body_excerpt: &str,
    latency_ms: i32,
    next_retry_at: Option<DateTime<Utc>>,
    circuit_breaker_threshold: i32,
) -> Result<bool, AppError> {
    let final_status = if next_retry_at.is_some() {
        "failed"
    } else {
        "dead_letter"
    };

    sqlx::query(
        r#"
        UPDATE webhook_deliveries
        SET
            status = $2,
            request_headers = $3,
            response_status = $4,
            response_body_excerpt = $5,
            latency_ms = $6,
            next_retry_at = NULL,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job.delivery_id)
    .bind(final_status)
    .bind(request_headers)
    .bind(response_status)
    .bind(response_body_excerpt)
    .bind(latency_ms)
    .execute(&mut **tx)
    .await?;

    if let Some(retry_time) = next_retry_at {
        sqlx::query(
            r#"
            INSERT INTO webhook_deliveries (
                environment_id, webhook_endpoint_id, outbox_event_id, attempt_number, status, next_retry_at
            )
            VALUES ($1, $2, $3, $4, 'retrying', $5)
            ON CONFLICT (webhook_endpoint_id, outbox_event_id, attempt_number) DO NOTHING
            "#,
        )
        .bind(job.environment_id)
        .bind(job.webhook_endpoint_id)
        .bind(job.outbox_event_id)
        .bind(job.attempt_number + 1)
        .bind(retry_time)
        .execute(&mut **tx)
        .await?;
    }

    let new_failures: i32 = sqlx::query_scalar(
        r#"
        UPDATE webhook_endpoints
        SET
            consecutive_failures = consecutive_failures + 1,
            last_failure_at = NOW(),
            status = CASE
                WHEN consecutive_failures + 1 >= $2 AND status = 'active' THEN 'paused'
                ELSE status
            END,
            paused_reason = CASE
                WHEN consecutive_failures + 1 >= $2 AND status = 'active' THEN 'auto_paused_consecutive_failures'
                ELSE paused_reason
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING consecutive_failures
        "#,
    )
    .bind(job.webhook_endpoint_id)
    .bind(circuit_breaker_threshold)
    .fetch_one(&mut **tx)
    .await?;

    Ok(new_failures >= circuit_breaker_threshold)
}

pub async fn mark_delivery_skipped_paused_in_tx(
    tx: &mut PgTx<'_>,
    delivery_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE webhook_deliveries
        SET status = 'dead_letter', response_body_excerpt = $2, next_retry_at = NULL, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn list_deliveries_for_endpoint(
    db: &PgPool,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
    limit: i64,
) -> Result<Vec<WebhookDeliveryRecord>, AppError> {
    let rows = sqlx::query_as::<_, WebhookDeliveryRecord>(
        r#"
        SELECT id, environment_id, webhook_endpoint_id, outbox_event_id, attempt_number,
               request_headers, response_status, response_body_excerpt, latency_ms, status,
               next_retry_at, delivered_at, created_at, updated_at
        FROM webhook_deliveries
        WHERE environment_id = $1 AND webhook_endpoint_id = $2
        ORDER BY created_at DESC, attempt_number DESC
        LIMIT $3
        "#,
    )
    .bind(environment_id)
    .bind(endpoint_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

pub async fn find_delivery_by_id<'e, E>(
    executor: E,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
    delivery_id: Uuid,
) -> Result<Option<WebhookDeliveryRecord>, AppError>
where
    E: PgExecutor<'e>,
{
    let row = sqlx::query_as::<_, WebhookDeliveryRecord>(
        r#"
        SELECT id, environment_id, webhook_endpoint_id, outbox_event_id, attempt_number,
               request_headers, response_status, response_body_excerpt, latency_ms, status,
               next_retry_at, delivered_at, created_at, updated_at
        FROM webhook_deliveries
        WHERE environment_id = $1 AND webhook_endpoint_id = $2 AND id = $3
        "#,
    )
    .bind(environment_id)
    .bind(endpoint_id)
    .bind(delivery_id)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

pub async fn enqueue_replay_delivery_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    endpoint_id: Uuid,
    outbox_event_id: Uuid,
) -> Result<WebhookDeliveryRecord, AppError> {
    let max_attempt: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(attempt_number), 0)
        FROM webhook_deliveries
        WHERE webhook_endpoint_id = $1 AND outbox_event_id = $2
        "#,
    )
    .bind(endpoint_id)
    .bind(outbox_event_id)
    .fetch_one(&mut **tx)
    .await?;

    let rec = sqlx::query_as::<_, WebhookDeliveryRecord>(
        r#"
        INSERT INTO webhook_deliveries (
            environment_id, webhook_endpoint_id, outbox_event_id, attempt_number, status, next_retry_at
        )
        VALUES ($1, $2, $3, $4, 'pending', NOW())
        RETURNING id, environment_id, webhook_endpoint_id, outbox_event_id, attempt_number,
                  request_headers, response_status, response_body_excerpt, latency_ms, status,
                  next_retry_at, delivered_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(endpoint_id)
    .bind(outbox_event_id)
    .bind(max_attempt + 1)
    .fetch_one(&mut **tx)
    .await?;

    Ok(rec)
}

pub async fn list_outbox_events_in_time_range(
    db: &PgPool,
    environment_id: EnvironmentId,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    event_types: Option<&[String]>,
    limit: i64,
) -> Result<Vec<OutboxEventRecord>, AppError> {
    let rows = if let Some(types) = event_types.filter(|t| !t.is_empty()) {
        sqlx::query_as::<_, OutboxEventRecord>(
            r#"
            SELECT id, environment_id, event_type, aggregate_type, aggregate_id,
                   aggregate_version, payload, status, available_at, attempts, created_at, updated_at
            FROM outbox_events
            WHERE environment_id = $1
              AND created_at >= $2
              AND created_at <= $3
              AND event_type = ANY($4)
            ORDER BY created_at ASC, id ASC
            LIMIT $5
            "#,
        )
        .bind(environment_id)
        .bind(from)
        .bind(to)
        .bind(types)
        .bind(limit)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as::<_, OutboxEventRecord>(
            r#"
            SELECT id, environment_id, event_type, aggregate_type, aggregate_id,
                   aggregate_version, payload, status, available_at, attempts, created_at, updated_at
            FROM outbox_events
            WHERE environment_id = $1
              AND created_at >= $2
              AND created_at <= $3
            ORDER BY created_at ASC, id ASC
            LIMIT $4
            "#,
        )
        .bind(environment_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(db)
        .await?
    };

    Ok(rows)
}
