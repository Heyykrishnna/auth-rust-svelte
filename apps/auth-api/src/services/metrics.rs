use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use sqlx::PgPool;

pub const FORBIDDEN_HIGH_CARDINALITY_LABELS: &[&str] = &[
    "user_id",
    "email",
    "environment_id",
    "sub",
    "ip",
    "ip_address",
    "token",
    "secret",
    "password",
];

pub const DOCUMENTED_METRIC_NAMES: &[&str] = &[
    "auth_sign_in_total",
    "auth_sign_up_total",
    "auth_lockout_total",
    "auth_latency_seconds",
    "oidc_authorize_total",
    "oidc_sso_hit_total",
    "oidc_token_exchange_total",
    "jwks_requests_total",
    "session_created_total",
    "session_revoked_total",
    "refresh_token_reuse_detected_total",
    "outbox_pending_depth",
    "outbox_oldest_age_seconds",
    "webhook_delivery_total",
    "webhook_delivery_latency_seconds",
    "webhook_dead_letter_total",
    "webhook_ssrf_blocked_total",
    "db_pool_connections_active",
    "db_pool_connections_idle",
    "redis_operation_latency_seconds",
    "redis_errors_total",
    "argon2_hash_duration_seconds",
    "signing_key_min_expiry_seconds",
    "http_plane_requests_total",
    "http_plane_request_duration_seconds",
];

pub fn is_safe_metric_label(label_name: &str) -> bool {
    let lower = label_name.trim().to_lowercase();
    !lower.is_empty() && !FORBIDDEN_HIGH_CARDINALITY_LABELS.contains(&lower.as_str())
}

pub fn init_metrics_descriptors() {
    describe_counter!(
        "auth_sign_in_total",
        "Total sign-in attempts by method and outcome"
    );
    describe_counter!("auth_sign_up_total", "Total sign-up attempts by outcome");
    describe_counter!(
        "auth_lockout_total",
        "Total brute-force authentication lockouts"
    );
    describe_histogram!(
        "auth_latency_seconds",
        "Latency of authentication operations in seconds"
    );
    describe_counter!(
        "oidc_authorize_total",
        "Total OIDC authorization requests by policy outcome"
    );
    describe_counter!("oidc_sso_hit_total", "Total silent cross-app OIDC SSO hits");
    describe_counter!(
        "oidc_token_exchange_total",
        "Total OIDC token exchanges by grant type and outcome"
    );
    describe_counter!("jwks_requests_total", "Total JWKS endpoint requests");
    describe_counter!(
        "session_created_total",
        "Total application sessions created"
    );
    describe_counter!("session_revoked_total", "Total sessions revoked by reason");
    describe_counter!(
        "refresh_token_reuse_detected_total",
        "Total refresh token reuse security anomalies detected"
    );
    describe_gauge!(
        "outbox_pending_depth",
        "Current count of unprocessed outbox events"
    );
    describe_gauge!(
        "outbox_oldest_age_seconds",
        "Age in seconds of the oldest unprocessed outbox event"
    );
    describe_counter!(
        "webhook_delivery_total",
        "Total outbound webhook deliveries by status class"
    );
    describe_histogram!(
        "webhook_delivery_latency_seconds",
        "Latency of outbound webhook deliveries in seconds"
    );
    describe_counter!(
        "webhook_dead_letter_total",
        "Total webhook deliveries moved to dead letter queue"
    );
    describe_counter!(
        "webhook_ssrf_blocked_total",
        "Total outbound webhook requests blocked by SSRF defenses"
    );
    describe_gauge!(
        "db_pool_connections_active",
        "Current active PostgreSQL pool connections"
    );
    describe_gauge!(
        "db_pool_connections_idle",
        "Current idle PostgreSQL pool connections"
    );
    describe_histogram!(
        "redis_operation_latency_seconds",
        "Latency of Redis operations in seconds"
    );
    describe_counter!("redis_errors_total", "Total Redis operation errors");
    describe_histogram!(
        "argon2_hash_duration_seconds",
        "Duration of Argon2id hashing and verification"
    );
    describe_gauge!(
        "signing_key_min_expiry_seconds",
        "Seconds until earliest active signing key expiration"
    );
    describe_counter!(
        "http_plane_requests_total",
        "HTTP requests classified by architectural plane and status class"
    );
    describe_histogram!(
        "http_plane_request_duration_seconds",
        "HTTP request latency by architectural plane"
    );

    counter!("auth_sign_in_total", "method" => "password", "outcome" => "success").increment(0);
    counter!("auth_sign_in_total", "method" => "password", "outcome" => "failure").increment(0);
    counter!("auth_sign_up_total", "outcome" => "initiated").increment(0);
    counter!("auth_sign_up_total", "outcome" => "verified").increment(0);
    counter!("auth_lockout_total").increment(0);
    histogram!("auth_latency_seconds", "operation" => "sign_in").record(0.0);
    counter!("oidc_authorize_total", "policy_outcome" => "issue_code").increment(0);
    counter!("oidc_sso_hit_total").increment(0);
    counter!("oidc_token_exchange_total", "grant_type" => "authorization_code", "outcome" => "success").increment(0);
    counter!("jwks_requests_total").increment(0);
    counter!("session_created_total").increment(0);
    counter!("session_revoked_total", "reason" => "user_logout").increment(0);
    counter!("refresh_token_reuse_detected_total").increment(0);
    gauge!("outbox_pending_depth").set(0.0);
    gauge!("outbox_oldest_age_seconds").set(0.0);
    counter!("webhook_delivery_total", "status_class" => "2xx").increment(0);
    histogram!("webhook_delivery_latency_seconds").record(0.0);
    counter!("webhook_dead_letter_total").increment(0);
    counter!("webhook_ssrf_blocked_total").increment(0);
    gauge!("db_pool_connections_active").set(0.0);
    gauge!("db_pool_connections_idle").set(0.0);
    histogram!("redis_operation_latency_seconds", "operation" => "get").record(0.0);
    counter!("redis_errors_total").increment(0);
    histogram!("argon2_hash_duration_seconds").record(0.0);
    gauge!("signing_key_min_expiry_seconds").set(86400.0 * 90.0);
    counter!("http_plane_requests_total", "plane" => "client", "status_class" => "2xx")
        .increment(0);
    histogram!("http_plane_request_duration_seconds", "plane" => "client").record(0.0);
}

pub fn record_sign_in(method: &'static str, outcome: &'static str, duration_secs: f64) {
    counter!("auth_sign_in_total", "method" => method, "outcome" => outcome).increment(1);
    histogram!("auth_latency_seconds", "operation" => "sign_in").record(duration_secs);
}

pub fn record_sign_up(outcome: &'static str) {
    counter!("auth_sign_up_total", "outcome" => outcome).increment(1);
}

pub fn record_lockout() {
    counter!("auth_lockout_total").increment(1);
}

pub fn record_oidc_authorize(policy_outcome: &'static str) {
    counter!("oidc_authorize_total", "policy_outcome" => policy_outcome).increment(1);
}

pub fn record_oidc_sso_hit() {
    counter!("oidc_sso_hit_total").increment(1);
}

pub fn record_oidc_token_exchange(grant_type: &'static str, outcome: &'static str) {
    counter!(
        "oidc_token_exchange_total",
        "grant_type" => grant_type,
        "outcome" => outcome
    )
    .increment(1);
}

pub fn record_jwks_request() {
    counter!("jwks_requests_total").increment(1);
}

pub fn record_session_created() {
    counter!("session_created_total").increment(1);
}

pub fn record_session_revoked(reason: &'static str) {
    counter!("session_revoked_total", "reason" => reason).increment(1);
}

pub fn record_refresh_token_reuse() {
    counter!("refresh_token_reuse_detected_total").increment(1);
}

pub fn set_outbox_backlog(pending_depth: i64, oldest_age_secs: f64) {
    gauge!("outbox_pending_depth").set(pending_depth.max(0) as f64);
    gauge!("outbox_oldest_age_seconds").set(oldest_age_secs.max(0.0));
}

pub fn record_webhook_delivery(status_class: &'static str, latency_secs: f64) {
    counter!("webhook_delivery_total", "status_class" => status_class).increment(1);
    histogram!("webhook_delivery_latency_seconds").record(latency_secs.max(0.0));
}

pub fn record_webhook_dead_letter() {
    counter!("webhook_dead_letter_total").increment(1);
}

pub fn record_webhook_ssrf_blocked() {
    counter!("webhook_ssrf_blocked_total").increment(1);
}

pub fn record_argon2_duration(duration_secs: f64) {
    histogram!("argon2_hash_duration_seconds").record(duration_secs.max(0.0));
}

pub fn record_redis_op(operation: &'static str, duration_secs: f64, is_error: bool) {
    histogram!("redis_operation_latency_seconds", "operation" => operation)
        .record(duration_secs.max(0.0));
    if is_error {
        counter!("redis_errors_total").increment(1);
    }
}

pub fn record_http_plane_request(plane: &'static str, status_code: u16, duration_secs: f64) {
    let status_class = match status_code {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    };
    counter!(
        "http_plane_requests_total",
        "plane" => plane,
        "status_class" => status_class
    )
    .increment(1);
    histogram!("http_plane_request_duration_seconds", "plane" => plane)
        .record(duration_secs.max(0.0));
}

pub fn sample_db_pool_metrics(db: &PgPool) {
    let size = db.size();
    let idle = db.num_idle() as u32;
    let active = size.saturating_sub(idle);
    gauge!("db_pool_connections_active").set(f64::from(active));
    gauge!("db_pool_connections_idle").set(f64::from(idle));
}

pub async fn refresh_infrastructure_gauges(db: &PgPool) {
    sample_db_pool_metrics(db);

    if let Ok(row) = sqlx::query_as::<_, (i64, f64)>(
        r#"
        SELECT
            COUNT(*)::bigint AS pending_depth,
            COALESCE(EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::float8, 0.0) AS oldest_age_secs
        FROM outbox_events
        WHERE status IN ('pending', 'processing')
        "#,
    )
    .fetch_one(db)
    .await
    {
        set_outbox_backlog(row.0, row.1);
    }

    if let Ok(Some((Some(secs),))) = sqlx::query_as::<_, (Option<f64>,)>(
        r#"
        SELECT MIN(EXTRACT(EPOCH FROM (retires_at - NOW()))::float8)
        FROM signing_keys
        WHERE status = 'active' AND retires_at IS NOT NULL
        "#,
    )
    .fetch_optional(db)
    .await
    {
        gauge!("signing_key_min_expiry_seconds").set(secs);
    }
}
