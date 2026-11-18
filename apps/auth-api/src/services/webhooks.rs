use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use rand_core::{OsRng, RngCore};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect::Policy;
use reqwest::Url;
use ring::hmac;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use tracing::{info, info_span, warn, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::tracing as otel_tracing;
use crate::models::{
    AggregateType, ApplicationId, DeveloperId, DomainEventEnvelope, DomainEventType, EnvironmentId,
    EnvironmentKind, WorkspaceRole,
};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::outbox;
use crate::repositories::tenants as tenant_repo;
use crate::repositories::webhooks::{
    self as webhook_repo, DueDeliveryJob, WebhookDeliveryRecord, WebhookEndpointRecord,
};
use crate::services::control_plane::require_application_access;
use crate::services::metrics as platform_metrics;
use crate::services::signing_keys::{
    decrypt_envelope, encrypt_envelope, WEBHOOK_SECRET_AAD_PURPOSE,
};

pub const WEBHOOK_EVENT_ID_HEADER: &str = "X-Auth-Event-Id";
pub const WEBHOOK_TIMESTAMP_HEADER: &str = "X-Auth-Timestamp";
pub const WEBHOOK_SIGNATURE_HEADER: &str = "X-Auth-Signature";
pub const DEFAULT_SIGNATURE_TOLERANCE_SECS: i64 = 300;
pub const MAX_RESPONSE_EXCERPT_BYTES: usize = 4096;
pub const CIRCUIT_BREAKER_FAILURE_THRESHOLD: i32 = 10;

const TRANSIENT_RETRY_DELAYS_SECS: &[i64] = &[10, 30, 120, 600, 1800, 7200, 21600, 43200];
const PERMANENT_4XX_RETRY_DELAYS_SECS: &[i64] = &[30, 120, 600];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpointView {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub url: String,
    pub description: String,
    pub subscribed_events: Vec<String>,
    pub status: String,
    pub consecutive_failures: i32,
    pub paused_reason: Option<String>,
    pub has_retiring_secret: bool,
    pub previous_secret_expires_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<WebhookEndpointRecord> for WebhookEndpointView {
    fn from(r: WebhookEndpointRecord) -> Self {
        let now = Utc::now();
        let has_retiring_secret = r.previous_signing_secret_encrypted.is_some()
            && r.previous_secret_expires_at
                .map(|exp| exp > now)
                .unwrap_or(false);
        Self {
            id: r.id,
            environment_id: r.environment_id,
            url: r.url,
            description: r.description,
            subscribed_events: r.subscribed_events,
            status: r.status,
            consecutive_failures: r.consecutive_failures,
            paused_reason: r.paused_reason,
            has_retiring_secret,
            previous_secret_expires_at: r.previous_secret_expires_at,
            last_failure_at: r.last_failure_at,
            last_success_at: r.last_success_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedWebhookEndpointResponse {
    pub endpoint: WebhookEndpointView,
    pub signing_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotatedWebhookSecretResponse {
    pub endpoint: WebhookEndpointView,
    pub signing_secret: String,
    pub previous_secret_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerTickStats {
    pub outbox_events_claimed: usize,
    pub deliveries_enqueued: usize,
    pub deliveries_processed: usize,
    pub deliveries_succeeded: usize,
    pub deliveries_failed: usize,
    pub deliveries_dead_lettered: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CreateWebhookInput {
    pub url: String,
    pub description: Option<String>,
    pub subscribed_events: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateWebhookInput {
    pub url: Option<String>,
    pub description: Option<String>,
    pub subscribed_events: Option<Vec<String>>,
    pub status: Option<String>,
}

pub fn generate_webhook_signing_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("whsec_{}", hex::encode(bytes))
}

pub fn compute_v1_signature(secret: &str, timestamp: i64, raw_body: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let mut ctx = hmac::Context::with_key(&key);
    ctx.update(timestamp.to_string().as_bytes());
    ctx.update(b".");
    ctx.update(raw_body);
    let tag = ctx.sign();
    format!("v1={}", hex::encode(tag.as_ref()))
}

pub fn build_signature_header(secrets: &[&str], timestamp: i64, raw_body: &[u8]) -> String {
    secrets
        .iter()
        .map(|secret| compute_v1_signature(secret, timestamp, raw_body))
        .collect::<Vec<_>>()
        .join(",")
}

pub fn verify_signature_header(
    signature_header: &str,
    secrets: &[&str],
    timestamp: i64,
    raw_body: &[u8],
    now_unix: i64,
    max_skew_secs: i64,
) -> Result<(), AppError> {
    if (now_unix - timestamp).abs() > max_skew_secs {
        return Err(AppError::Unauthorized(
            "Webhook timestamp outside allowed tolerance window".to_string(),
        ));
    }

    let presented: Vec<&str> = signature_header
        .split(',')
        .map(str::trim)
        .filter(|part| part.starts_with("v1="))
        .collect();

    if presented.is_empty() {
        return Err(AppError::Unauthorized(
            "Missing v1 webhook signature".to_string(),
        ));
    }

    for secret in secrets {
        let expected = compute_v1_signature(secret, timestamp, raw_body);
        let expected_bytes = expected.as_bytes();
        for candidate in &presented {
            let cand_bytes = candidate.as_bytes();
            if cand_bytes.len() == expected_bytes.len()
                && bool::from(cand_bytes.ct_eq(expected_bytes))
            {
                return Ok(());
            }
        }
    }

    Err(AppError::Unauthorized(
        "Webhook signature verification failed".to_string(),
    ))
}

pub fn is_blocked_ipv4(ip: Ipv4Addr, allow_loopback: bool) -> bool {
    let octets = ip.octets();
    if ip.is_loopback() {
        return !allow_loopback;
    }
    if ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
    {
        return true;
    }
    if octets[0] == 0 {
        return true;
    }
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return true;
    }
    if octets[0] >= 240 {
        return true;
    }
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return true;
    }
    false
}

pub fn is_blocked_ipv6(ip: Ipv6Addr, allow_loopback: bool) -> bool {
    if let Some(mapped_v4) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(mapped_v4, allow_loopback);
    }
    if ip.is_loopback() {
        return !allow_loopback;
    }
    if ip.is_unspecified() || ip.is_multicast() {
        return true;
    }
    let segments = ip.segments();
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    if (segments[0] & 0xffc0) == 0xfec0 {
        return true;
    }
    false
}

pub fn is_blocked_ip(ip: IpAddr, allow_loopback: bool) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4, allow_loopback),
        IpAddr::V6(v6) => is_blocked_ipv6(v6, allow_loopback),
    }
}

pub fn validate_webhook_url(raw_url: &str, env_kind: EnvironmentKind) -> Result<Url, AppError> {
    let trimmed = raw_url.trim();
    let parsed = Url::parse(trimmed)
        .map_err(|e| AppError::Validation(format!("Invalid webhook URL: {e}")))?;

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AppError::Validation(
            "Webhook URL must not embed credentials".to_string(),
        ));
    }

    let host_str = parsed
        .host_str()
        .ok_or_else(|| AppError::Validation("Webhook URL must include a host".to_string()))?
        .to_lowercase();
    let host_unbracketed = host_str.trim_start_matches('[').trim_end_matches(']');

    let allow_loopback = env_kind == EnvironmentKind::Development;
    let is_loopback_host = host_unbracketed == "localhost"
        || host_unbracketed == "127.0.0.1"
        || host_unbracketed == "::1";

    match parsed.scheme() {
        "https" => {}
        "http" if allow_loopback && is_loopback_host => {}
        "http" => {
            return Err(AppError::Validation(
                "Webhook URL must use https (http://localhost is allowed only in development)"
                    .to_string(),
            ));
        }
        other => {
            return Err(AppError::Validation(format!(
                "Unsupported webhook URL scheme '{other}'"
            )));
        }
    }

    if !allow_loopback
        && (is_loopback_host
            || host_unbracketed.ends_with(".localhost")
            || host_unbracketed.ends_with(".local")
            || host_unbracketed.ends_with(".internal")
            || host_unbracketed == "metadata.google.internal")
    {
        return Err(AppError::Validation(
            "Webhook URL host is blocked by SSRF policy".to_string(),
        ));
    }

    if let Ok(ip) = host_unbracketed.parse::<IpAddr>() {
        if is_blocked_ip(ip, allow_loopback) {
            return Err(AppError::Validation(format!(
                "Webhook IP address '{ip}' is blocked by SSRF policy"
            )));
        }
    }

    Ok(parsed)
}

#[derive(Debug, Clone)]
pub struct SsrfSafeDnsResolver {
    pub allow_loopback: bool,
}

impl Resolve for SsrfSafeDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let allow_loopback = self.allow_loopback;

        Box::pin(async move {
            let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                .collect();

            if resolved.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "DNS resolution returned no addresses",
                ))
                    as Box<dyn std::error::Error + Send + Sync>);
            }

            for addr in &resolved {
                if is_blocked_ip(addr.ip(), allow_loopback) {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!(
                            "Resolved webhook IP {} is blocked by SSRF policy",
                            addr.ip()
                        ),
                    ))
                        as Box<dyn std::error::Error + Send + Sync>);
                }
            }

            let addrs: Addrs = Box::new(resolved.into_iter());
            Ok(addrs)
        })
    }
}

pub fn build_ssrf_safe_http_client(allow_loopback: bool) -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .dns_resolver(Arc::new(SsrfSafeDnsResolver { allow_loopback }))
        .redirect(Policy::none())
        .connect_timeout(StdDuration::from_secs(3))
        .timeout(StdDuration::from_secs(10))
        .user_agent("SecureAuth-WebhookWorker/1.0")
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build webhook HTTP client: {e}")))
}

pub fn is_permanent_4xx(status: Option<i32>) -> bool {
    match status {
        Some(code) => (400..500).contains(&code) && code != 408 && code != 429,
        None => false,
    }
}

pub fn compute_next_retry_delay_secs(attempt_number: i32, permanent_4xx: bool) -> Option<i64> {
    if attempt_number < 1 {
        return Some(TRANSIENT_RETRY_DELAYS_SECS[0]);
    }
    let idx = (attempt_number - 1) as usize;
    let schedule = if permanent_4xx {
        PERMANENT_4XX_RETRY_DELAYS_SECS
    } else {
        TRANSIENT_RETRY_DELAYS_SECS
    };
    schedule.get(idx).copied()
}

fn apply_jitter(base_secs: i64) -> i64 {
    let max_delta = (base_secs / 10).max(1);
    let delta: i64 = rand::thread_rng().gen_range(-max_delta..=max_delta);
    (base_secs + delta).max(1)
}

fn truncate_excerpt(body: &str) -> String {
    if body.len() <= MAX_RESPONSE_EXCERPT_BYTES {
        return body.to_string();
    }
    let mut end = MAX_RESPONSE_EXCERPT_BYTES;
    while !body.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    body[..end].to_string()
}

fn decrypt_active_and_grace_secrets(
    job: &DueDeliveryJob,
    config: &AppConfig,
) -> Result<zeroize::Zeroizing<Vec<String>>, AppError> {
    let active_bytes = decrypt_envelope(
        &job.signing_secret_encrypted,
        &config.signing_key_kek,
        &job.kek_version,
        WEBHOOK_SECRET_AAD_PURPOSE,
    )?;
    let active_secret = String::from_utf8(active_bytes.to_vec())
        .map_err(|_| AppError::Internal("Corrupted webhook signing secret UTF-8".to_string()))?;

    let mut secrets = vec![active_secret];
    if let (Some(prev_enc), Some(prev_exp)) = (
        job.previous_signing_secret_encrypted.as_ref(),
        job.previous_secret_expires_at,
    ) {
        if prev_exp > Utc::now() {
            if let Ok(prev_bytes) = decrypt_envelope(
                prev_enc,
                &config.signing_key_kek,
                &job.kek_version,
                WEBHOOK_SECRET_AAD_PURPOSE,
            ) {
                if let Ok(prev_secret) = String::from_utf8(prev_bytes.to_vec()) {
                    secrets.push(prev_secret);
                }
            }
        }
    }

    Ok(zeroize::Zeroizing::new(secrets))
}

pub async fn dispatch_outbox_to_deliveries(
    db: &PgPool,
    batch_size: i64,
) -> Result<(usize, usize), AppError> {
    let mut tx = begin_tx(db).await?;
    let events = webhook_repo::claim_pending_outbox_events_in_tx(&mut tx, batch_size).await?;
    let claimed_count = events.len();
    let mut enqueued = 0usize;

    for event in events {
        let endpoints = webhook_repo::find_active_matching_endpoints_in_tx(
            &mut tx,
            event.environment_id,
            &event.event_type,
        )
        .await?;

        for ep in endpoints {
            if webhook_repo::enqueue_delivery_in_tx(
                &mut tx,
                event.environment_id,
                ep.id,
                event.id,
                1,
            )
            .await?
            .is_some()
            {
                enqueued += 1;
            }
        }

        webhook_repo::mark_outbox_event_published_in_tx(&mut tx, event.id).await?;
    }

    commit_tx(tx).await?;
    Ok((claimed_count, enqueued))
}

struct DeliveryAttemptOutcome {
    succeeded: bool,
    response_status: Option<i32>,
    response_excerpt: String,
    latency_ms: i32,
    request_headers: serde_json::Value,
    permanent_4xx: bool,
}

async fn execute_single_delivery(
    job: &DueDeliveryJob,
    config: &AppConfig,
) -> Result<DeliveryAttemptOutcome, AppError> {
    let env_kind =
        EnvironmentKind::from_db_str(&job.environment_kind).unwrap_or(EnvironmentKind::Production);
    let allow_loopback = env_kind == EnvironmentKind::Development;

    let validated_url = match validate_webhook_url(&job.endpoint_url, env_kind) {
        Ok(u) => u,
        Err(err) => {
            platform_metrics::record_webhook_ssrf_blocked();
            platform_metrics::record_webhook_delivery("blocked", 0.0);
            return Ok(DeliveryAttemptOutcome {
                succeeded: false,
                response_status: None,
                response_excerpt: truncate_excerpt(&err.to_string()),
                latency_ms: 0,
                request_headers: serde_json::json!({}),
                permanent_4xx: true,
            });
        }
    };

    let secrets = decrypt_active_and_grace_secrets(job, config)?;
    let secret_refs: Vec<&str> = secrets.iter().map(String::as_str).collect();
    let raw_body = serde_json::to_vec(&job.event_payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize webhook payload: {e}")))?;

    let timestamp = Utc::now().timestamp();
    let signature_header = build_signature_header(&secret_refs, timestamp, &raw_body);
    let event_id_header = job
        .event_payload
        .get("event_id")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| job.outbox_event_id.to_string());

    let req_headers_json = serde_json::json!({
        "Content-Type": "application/json",
        WEBHOOK_EVENT_ID_HEADER: event_id_header,
        WEBHOOK_TIMESTAMP_HEADER: timestamp.to_string(),
        WEBHOOK_SIGNATURE_HEADER: signature_header,
    });

    let client = shared_webhook_client(allow_loopback)?;
    let started = Instant::now();

    let mut req_builder = client
        .post(validated_url)
        .header("Content-Type", "application/json")
        .header(WEBHOOK_EVENT_ID_HEADER, &event_id_header)
        .header(WEBHOOK_TIMESTAMP_HEADER, timestamp.to_string())
        .header(WEBHOOK_SIGNATURE_HEADER, &signature_header);
    if let Some(tp) = otel_tracing::current_traceparent().or_else(|| job.event_traceparent.clone())
    {
        req_builder = req_builder.header("traceparent", tp);
    }

    let send_res = req_builder.body(raw_body).send().await;

    let elapsed_secs = started.elapsed().as_secs_f64();
    let latency_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;

    match send_res {
        Ok(resp) => {
            let status_code = resp.status().as_u16() as i32;
            if (300..400).contains(&status_code) {
                platform_metrics::record_webhook_ssrf_blocked();
                platform_metrics::record_webhook_delivery("3xx", elapsed_secs);
                return Ok(DeliveryAttemptOutcome {
                    succeeded: false,
                    response_status: Some(status_code),
                    response_excerpt: format!(
                        "Redirect response ({status_code}) blocked by SSRF policy"
                    ),
                    latency_ms,
                    request_headers: req_headers_json,
                    permanent_4xx: true,
                });
            }

            let status_class = match status_code {
                200..=299 => "2xx",
                400..=499 => "4xx",
                500..=599 => "5xx",
                _ => "other",
            };
            platform_metrics::record_webhook_delivery(status_class, elapsed_secs);

            let body_text = resp.text().await.unwrap_or_default();
            let excerpt = truncate_excerpt(&body_text);
            let succeeded = (200..300).contains(&status_code);
            let perm_4xx = is_permanent_4xx(Some(status_code));

            Ok(DeliveryAttemptOutcome {
                succeeded,
                response_status: Some(status_code),
                response_excerpt: excerpt,
                latency_ms,
                request_headers: req_headers_json,
                permanent_4xx: perm_4xx,
            })
        }
        Err(err) => {
            platform_metrics::record_webhook_delivery("network_error", elapsed_secs);
            Ok(DeliveryAttemptOutcome {
                succeeded: false,
                response_status: None,
                response_excerpt: truncate_excerpt(&err.to_string()),
                latency_ms,
                request_headers: req_headers_json,
                permanent_4xx: false,
            })
        }
    }
}

pub async fn run_worker_tick(
    db: &PgPool,
    config: &AppConfig,
    batch_size: i64,
) -> Result<WorkerTickStats, AppError> {
    let mut stats = WorkerTickStats::default();

    let (claimed, enqueued) = dispatch_outbox_to_deliveries(db, batch_size).await?;
    stats.outbox_events_claimed = claimed;
    stats.deliveries_enqueued = enqueued;

    let mut claim_tx = begin_tx(db).await?;
    let jobs = webhook_repo::claim_due_deliveries_in_tx(&mut claim_tx, batch_size).await?;

    for job in jobs {
        stats.deliveries_processed += 1;

        if job.endpoint_status != "active" {
            webhook_repo::mark_delivery_skipped_paused_in_tx(
                &mut claim_tx,
                job.delivery_id,
                &format!("Endpoint status is '{}'", job.endpoint_status),
            )
            .await?;
            stats.deliveries_dead_lettered += 1;
            continue;
        }

        let delivery_span = info_span!(
            "webhook_delivery",
            delivery_id = %job.delivery_id,
            outbox_event_id = %job.outbox_event_id,
            event_type = %job.event_type,
            attempt = job.attempt_number
        );
        if let Some(tp) = job.event_traceparent.as_deref() {
            delivery_span.set_parent(otel_tracing::extract_context_from_traceparent(tp));
        }

        let outcome = execute_single_delivery(&job, config)
            .instrument(delivery_span)
            .await?;
        if outcome.succeeded {
            webhook_repo::mark_delivery_succeeded_in_tx(
                &mut claim_tx,
                job.delivery_id,
                job.webhook_endpoint_id,
                &outcome.request_headers,
                outcome.response_status.unwrap_or(200),
                &outcome.response_excerpt,
                outcome.latency_ms,
            )
            .await?;
            stats.deliveries_succeeded += 1;
        } else {
            let next_retry_at =
                compute_next_retry_delay_secs(job.attempt_number, outcome.permanent_4xx)
                    .map(|base| Utc::now() + Duration::seconds(apply_jitter(base)));

            let auto_paused = webhook_repo::mark_delivery_failed_and_schedule_next_in_tx(
                &mut claim_tx,
                &job,
                &outcome.request_headers,
                outcome.response_status,
                &outcome.response_excerpt,
                outcome.latency_ms,
                next_retry_at,
                CIRCUIT_BREAKER_FAILURE_THRESHOLD,
            )
            .await?;

            if auto_paused {
                warn!(
                    endpoint_id = %job.webhook_endpoint_id,
                    environment_id = %job.environment_id,
                    "Auto-paused webhook endpoint after consecutive delivery failures"
                );
                sqlx::query(
                    r#"
                    INSERT INTO audit_logs (
                        environment_id, actor_type, event, resource_type, resource_id, metadata
                    )
                    VALUES ($1, 'system', 'WEBHOOK_ENDPOINT_AUTO_PAUSED', 'webhook_endpoint', $2, $3)
                    "#,
                )
                .bind(job.environment_id)
                .bind(job.webhook_endpoint_id)
                .bind(serde_json::json!({
                    "endpoint_id": job.webhook_endpoint_id,
                    "threshold": CIRCUIT_BREAKER_FAILURE_THRESHOLD
                }))
                .execute(&mut *claim_tx)
                .await?;
            }

            if next_retry_at.is_some() {
                stats.deliveries_failed += 1;
            } else {
                stats.deliveries_dead_lettered += 1;
                platform_metrics::record_webhook_dead_letter();
            }
        }
    }

    commit_tx(claim_tx).await?;
    platform_metrics::refresh_infrastructure_gauges(db).await;
    Ok(stats)
}

async fn resolve_developer_environment(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    min_role: WorkspaceRole,
) -> Result<(EnvironmentId, EnvironmentKind), AppError> {
    let _ = require_application_access(db, developer_id, app_id, min_role).await?;
    let env = tenant_repo::find_environment_by_app_and_kind(db, app_id, env_kind)
        .await?
        .ok_or_else(|| AppError::Validation("Environment not found".to_string()))?;
    Ok((env.id, env_kind))
}

pub async fn list_webhook_endpoints(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
) -> Result<Vec<WebhookEndpointView>, AppError> {
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Viewer)
            .await?;
    let rows = webhook_repo::list_endpoints_for_environment(db, env_id).await?;
    Ok(rows.into_iter().map(WebhookEndpointView::from).collect())
}

pub async fn create_webhook_endpoint(
    db: &PgPool,
    config: &AppConfig,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    input: CreateWebhookInput,
) -> Result<CreatedWebhookEndpointResponse, AppError> {
    let (env_id, kind) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Developer)
            .await?;

    let validated = validate_webhook_url(&input.url, kind)?;
    let raw_secret = generate_webhook_signing_secret();
    let encrypted = encrypt_envelope(
        raw_secret.as_bytes(),
        &config.signing_key_kek,
        &config.signing_key_kek_version,
        WEBHOOK_SECRET_AAD_PURPOSE,
    )?;

    let subscribed: Vec<String> = input
        .subscribed_events
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let description = input.description.unwrap_or_default().trim().to_string();

    let mut tx = begin_tx(db).await?;
    let record = webhook_repo::create_endpoint(
        &mut *tx,
        env_id,
        validated.as_str(),
        &description,
        &subscribed,
        &encrypted,
        &config.signing_key_kek_version,
    )
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env_id),
        developer_id,
        "WEBHOOK_ENDPOINT_CREATED",
        "webhook_endpoint",
        Some(record.id),
        serde_json::json!({ "url": record.url, "subscribed_events": record.subscribed_events }),
    )
    .await?;
    commit_tx(tx).await?;

    info!(endpoint_id = %record.id, environment_id = %env_id, "Created webhook endpoint");
    Ok(CreatedWebhookEndpointResponse {
        endpoint: WebhookEndpointView::from(record),
        signing_secret: raw_secret,
    })
}

pub async fn update_webhook_endpoint(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
    input: UpdateWebhookInput,
) -> Result<WebhookEndpointView, AppError> {
    let (env_id, kind) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Developer)
            .await?;

    let validated_url = match input.url.as_deref() {
        Some(raw) => Some(validate_webhook_url(raw, kind)?.to_string()),
        None => None,
    };

    if let Some(ref st) = input.status {
        if !matches!(st.as_str(), "active" | "paused" | "disabled") {
            return Err(AppError::Validation(
                "Webhook status must be 'active', 'paused', or 'disabled'".to_string(),
            ));
        }
    }

    let mut tx = begin_tx(db).await?;
    let updated = webhook_repo::update_endpoint(
        &mut *tx,
        env_id,
        endpoint_id,
        validated_url.as_deref(),
        input.description.as_deref().map(str::trim),
        input.subscribed_events.as_deref(),
        input.status.as_deref(),
    )
    .await?
    .ok_or_else(|| AppError::Validation("Webhook endpoint not found".to_string()))?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env_id),
        developer_id,
        "WEBHOOK_ENDPOINT_UPDATED",
        "webhook_endpoint",
        Some(updated.id),
        serde_json::json!({ "status": updated.status, "url": updated.url }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(WebhookEndpointView::from(updated))
}

pub async fn delete_webhook_endpoint(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
) -> Result<(), AppError> {
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Admin)
            .await?;

    let mut tx = begin_tx(db).await?;
    let deleted = webhook_repo::delete_endpoint(&mut *tx, env_id, endpoint_id).await?;
    if !deleted {
        return Err(AppError::Validation(
            "Webhook endpoint not found".to_string(),
        ));
    }
    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env_id),
        developer_id,
        "WEBHOOK_ENDPOINT_DELETED",
        "webhook_endpoint",
        Some(endpoint_id),
        serde_json::json!({}),
    )
    .await?;
    commit_tx(tx).await?;
    Ok(())
}

pub async fn rotate_webhook_endpoint_secret(
    db: &PgPool,
    config: &AppConfig,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
    grace_hours: Option<i64>,
) -> Result<RotatedWebhookSecretResponse, AppError> {
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Admin)
            .await?;

    let hours = grace_hours.unwrap_or(24).clamp(1, 168);
    let grace_expires_at = Utc::now() + Duration::hours(hours);
    let new_secret = generate_webhook_signing_secret();
    let encrypted = encrypt_envelope(
        new_secret.as_bytes(),
        &config.signing_key_kek,
        &config.signing_key_kek_version,
        WEBHOOK_SECRET_AAD_PURPOSE,
    )?;

    let mut tx = begin_tx(db).await?;
    let updated = webhook_repo::rotate_endpoint_secret(
        &mut *tx,
        env_id,
        endpoint_id,
        &encrypted,
        &config.signing_key_kek_version,
        grace_expires_at,
    )
    .await?
    .ok_or_else(|| AppError::Validation("Webhook endpoint not found".to_string()))?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env_id),
        developer_id,
        "WEBHOOK_SECRET_ROTATED",
        "webhook_endpoint",
        Some(updated.id),
        serde_json::json!({ "grace_expires_at": grace_expires_at }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(RotatedWebhookSecretResponse {
        endpoint: WebhookEndpointView::from(updated),
        signing_secret: new_secret,
        previous_secret_expires_at: grace_expires_at,
    })
}

pub async fn send_test_webhook_ping(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
) -> Result<WebhookDeliveryRecord, AppError> {
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Developer)
            .await?;

    let mut tx = begin_tx(db).await?;
    let ep = webhook_repo::find_endpoint_by_id(&mut *tx, env_id, endpoint_id)
        .await?
        .ok_or_else(|| AppError::Validation("Webhook endpoint not found".to_string()))?;

    let test_evt = DomainEventEnvelope::new(
        DomainEventType::WebhookTest,
        Some(env_id.into_inner()),
        AggregateType::WebhookEndpoint,
        ep.id,
        1,
        serde_json::json!({
            "endpoint_id": ep.id,
            "message": "Synthetic webhook.test ping"
        }),
    );
    let outbox_id = outbox::insert_outbox_event(&mut *tx, env_id, &test_evt).await?;
    webhook_repo::mark_outbox_event_published_in_tx(&mut tx, outbox_id).await?;
    let delivery =
        webhook_repo::enqueue_replay_delivery_in_tx(&mut tx, env_id, ep.id, outbox_id).await?;
    commit_tx(tx).await?;

    Ok(delivery)
}

pub async fn list_webhook_deliveries(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
    limit: Option<i64>,
) -> Result<Vec<WebhookDeliveryRecord>, AppError> {
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Viewer)
            .await?;
    let _ = webhook_repo::find_endpoint_by_id(db, env_id, endpoint_id)
        .await?
        .ok_or_else(|| AppError::Validation("Webhook endpoint not found".to_string()))?;
    let safe_limit = limit.unwrap_or(50).clamp(1, 200);
    webhook_repo::list_deliveries_for_endpoint(db, env_id, endpoint_id, safe_limit).await
}

pub async fn replay_single_delivery(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
    delivery_id: Uuid,
) -> Result<WebhookDeliveryRecord, AppError> {
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Developer)
            .await?;

    let mut tx = begin_tx(db).await?;
    let original = webhook_repo::find_delivery_by_id(&mut *tx, env_id, endpoint_id, delivery_id)
        .await?
        .ok_or_else(|| AppError::Validation("Webhook delivery not found".to_string()))?;

    let replayed = webhook_repo::enqueue_replay_delivery_in_tx(
        &mut tx,
        env_id,
        endpoint_id,
        original.outbox_event_id,
    )
    .await?;

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env_id),
        developer_id,
        "WEBHOOK_DELIVERY_REPLAYED",
        "webhook_delivery",
        Some(replayed.id),
        serde_json::json!({
            "original_delivery_id": delivery_id,
            "outbox_event_id": original.outbox_event_id
        }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(replayed)
}

#[allow(clippy::too_many_arguments)]
pub async fn replay_endpoint_time_range(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    env_kind: EnvironmentKind,
    endpoint_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    event_types: Option<Vec<String>>,
) -> Result<usize, AppError> {
    if to < from {
        return Err(AppError::Validation(
            "'to' timestamp must be greater than or equal to 'from'".to_string(),
        ));
    }
    let (env_id, _) =
        resolve_developer_environment(db, developer_id, app_id, env_kind, WorkspaceRole::Developer)
            .await?;

    let _ = webhook_repo::find_endpoint_by_id(db, env_id, endpoint_id)
        .await?
        .ok_or_else(|| AppError::Validation("Webhook endpoint not found".to_string()))?;

    let events = webhook_repo::list_outbox_events_in_time_range(
        db,
        env_id,
        from,
        to,
        event_types.as_deref(),
        500,
    )
    .await?;

    let mut tx = begin_tx(db).await?;
    let mut enqueued = 0usize;
    for evt in events {
        webhook_repo::enqueue_replay_delivery_in_tx(&mut tx, env_id, endpoint_id, evt.id).await?;
        enqueued += 1;
    }

    tenant_repo::record_control_audit_log(
        &mut *tx,
        Some(env_id),
        developer_id,
        "WEBHOOK_RANGE_REPLAYED",
        "webhook_endpoint",
        Some(endpoint_id),
        serde_json::json!({
            "from": from,
            "to": to,
            "enqueued_count": enqueued
        }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(enqueued)
}

fn shared_webhook_client(allow_loopback: bool) -> Result<reqwest::Client, AppError> {
    static PUBLIC_CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> = std::sync::OnceLock::new();
    static LOOPBACK_CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> = std::sync::OnceLock::new();
    let slot = if allow_loopback { &LOOPBACK_CLIENT } else { &PUBLIC_CLIENT };
    slot.get_or_init(|| build_ssrf_safe_http_client(allow_loopback).map_err(|error| error.to_string()))
        .as_ref().cloned().map_err(|error| AppError::Internal(error.clone()))
}
