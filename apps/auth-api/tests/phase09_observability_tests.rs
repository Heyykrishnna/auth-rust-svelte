use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use auth_api::middleware::tracing::{
    classify_plane, extract_context_from_traceparent, redact_sensitive_json, redact_sensitive_text,
};
use auth_api::services::analytics::{compute_webhook_success_rate, AnalyticsWindow};
use auth_api::services::metrics::{
    init_metrics_descriptors, is_safe_metric_label, record_argon2_duration,
    record_http_plane_request, record_jwks_request, record_lockout, record_oidc_authorize,
    record_oidc_sso_hit, record_oidc_token_exchange, record_redis_op, record_refresh_token_reuse,
    record_session_created, record_session_revoked, record_sign_in, record_sign_up,
    record_webhook_dead_letter, record_webhook_delivery, record_webhook_ssrf_blocked,
    set_outbox_backlog, DOCUMENTED_METRIC_NAMES, FORBIDDEN_HIGH_CARDINALITY_LABELS,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use opentelemetry::trace::TraceContextExt;
use tracing_subscriber::fmt;

#[derive(Clone, Default)]
struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self.0.lock().unwrap();
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> fmt::MakeWriter<'a> for BufferWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn test_pii_and_secret_redaction_in_structured_json_logs() {
    let fake_sk = format!("{}{}", "sk_", "live_9876543210abcdef");
    let fake_wh = format!("{}{}", "whsec_", "supersecretvalue123456");
    let fake_inv = format!("{}{}", "orginv_", "abc123def456ghi789");
    let fake_jwt = format!(
        "{}.{}.{}",
        "eyJhbGciOiJFUzI1NiJ9", "eyJzdWIiOiJ1c2VyXzEifQ", "signaturepart12345"
    );
    let fake_password = "CorrectHorseBatteryStaple99!";
    let fake_otp = "849201";

    let raw_line = format!(
        "auth failed key={fake_sk} webhook={fake_wh} invite={fake_inv} Authorization: Bearer {fake_jwt}"
    );
    let sanitized_line = redact_sensitive_text(&raw_line);
    assert!(!sanitized_line.contains(&fake_sk));
    assert!(!sanitized_line.contains(&fake_wh));
    assert!(!sanitized_line.contains(&fake_inv));
    assert!(!sanitized_line.contains(&fake_jwt));
    assert!(sanitized_line.contains("[REDACTED]"));

    let mut payload = serde_json::json!({
        "email": "user@example.test",
        "password": fake_password,
        "otp": fake_otp,
        "refresh_token": fake_jwt,
        "signing_secret": fake_wh,
        "nested": {
            "client_secret": fake_sk,
            "note": format!("header Bearer {fake_jwt}")
        }
    });
    redact_sensitive_json(&mut payload);

    let buffer = BufferWriter::default();
    let subscriber = fmt::Subscriber::builder()
        .json()
        .with_writer(buffer.clone())
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        tracing::warn!(
            plane = "client",
            sanitized_msg = %sanitized_line,
            sanitized_payload = %payload,
            "Simulated authentication failure"
        );
    });

    let captured = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(captured.trim()).unwrap();
    assert_eq!(parsed["fields"]["plane"], "client");
    assert!(!captured.contains(fake_password));
    assert!(!captured.contains(fake_otp));
    assert!(!captured.contains(&fake_sk));
    assert!(!captured.contains(&fake_wh));
    assert!(!captured.contains(&fake_inv));
    assert!(!captured.contains(&fake_jwt));
}

#[test]
fn test_prometheus_metrics_exposure_and_cardinality_guardrails() {
    let handle = PrometheusBuilder::new().install_recorder().unwrap();
    init_metrics_descriptors();

    record_sign_in("password", "success", 0.045);
    record_sign_in("google", "failure", 0.08);
    record_sign_up("verified");
    record_lockout();
    record_oidc_authorize("issue_code");
    record_oidc_sso_hit();
    record_oidc_token_exchange("authorization_code", "success");
    record_jwks_request();
    record_session_created();
    record_session_revoked("refresh_token_reuse");
    record_refresh_token_reuse();
    set_outbox_backlog(3, 12.5);
    record_webhook_delivery("2xx", 0.12);
    record_webhook_dead_letter();
    record_webhook_ssrf_blocked();
    record_argon2_duration(0.09);
    record_redis_op("get", 0.002, false);
    record_http_plane_request("client", 200, 0.015);

    let rendered = handle.render();

    for metric_name in DOCUMENTED_METRIC_NAMES {
        assert!(
            rendered.contains(metric_name),
            "Expected metric '{metric_name}' in Prometheus /metrics exposition"
        );
    }

    for forbidden in FORBIDDEN_HIGH_CARDINALITY_LABELS {
        assert!(!is_safe_metric_label(forbidden));
        let label_needle = format!("{forbidden}=\"");
        assert!(
            !rendered.contains(&label_needle),
            "Forbidden high-cardinality label '{forbidden}' leaked into Prometheus metrics"
        );
    }

    assert!(is_safe_metric_label("plane"));
    assert!(is_safe_metric_label("method"));
    assert!(is_safe_metric_label("outcome"));
    assert!(is_safe_metric_label("status_class"));
}

#[test]
fn test_plane_classification_traceparent_and_analytics_window_logic() {
    assert_eq!(classify_plane("/oauth2/authorize"), "oidc");
    assert_eq!(classify_plane("/.well-known/jwks.json"), "oidc");
    assert_eq!(classify_plane("/v1/client/auth/sign-in"), "client");
    assert_eq!(classify_plane("/v1/backend/users"), "backend");
    assert_eq!(classify_plane("/v1/control/applications"), "control");
    assert_eq!(classify_plane("/health"), "health");
    assert_eq!(classify_plane("/metrics"), "metrics");
    assert_eq!(classify_plane("/auth/login"), "legacy");

    let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    let cx = extract_context_from_traceparent(tp);
    let span_ctx = cx.span().span_context().clone();
    assert!(span_ctx.is_valid());
    assert_eq!(
        span_ctx.trace_id().to_string(),
        "4bf92f3577b34da6a3ce929d0e0e4736"
    );
    assert_eq!(span_ctx.span_id().to_string(), "00f067aa0ba902b7");

    assert_eq!(
        AnalyticsWindow::parse(None).unwrap(),
        AnalyticsWindow::Days7
    );
    assert_eq!(
        AnalyticsWindow::parse(Some("24h")).unwrap().granularity(),
        "hour"
    );
    assert_eq!(
        AnalyticsWindow::parse(Some("30d")).unwrap().bucket_count(),
        30
    );
    assert_eq!(
        AnalyticsWindow::parse(Some("90d")).unwrap().bucket_count(),
        90
    );
    assert!(AnalyticsWindow::parse(Some("365d")).is_err());

    assert_eq!(compute_webhook_success_rate(0, 0, 0), 100.0);
    assert_eq!(compute_webhook_success_rate(95, 3, 2), 95.0);
}

#[test]
fn test_prometheus_alert_rules_and_grafana_dashboards_are_valid() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let prom_cfg =
        std::fs::read_to_string(repo_root.join("observability/prometheus/prometheus-local.yaml"))
            .unwrap();
    assert!(prom_cfg.contains("/etc/prometheus/alerts.yml"));
    assert!(prom_cfg.contains("webhook-worker:9091"));

    let alerts_yaml =
        std::fs::read_to_string(repo_root.join("observability/prometheus/alerts.yml")).unwrap();
    for required_alert in [
        "HighHttp5xxBurnRate",
        "RefreshTokenReuseDetected",
        "OutboxBacklogAgeHigh",
        "WebhookDeadLetterSurge",
        "SigningKeyNearingExpiry",
        "PostgresPoolSaturation",
        "RedisErrorsDetected",
    ] {
        assert!(
            alerts_yaml.contains(&format!("alert: {required_alert}")),
            "Missing required Prometheus alert '{required_alert}'"
        );
    }

    let dashboards_dir = repo_root.join("observability/grafana/dashboards");
    let expected_dashboards = [
        "auth-dashboard.json",
        "oidc-tokens-dashboard.json",
        "webhooks-sync-dashboard.json",
        "database-redis-dashboard.json",
    ];
    let mut seen_uids = HashSet::new();
    for file in expected_dashboards {
        let raw = std::fs::read_to_string(dashboards_dir.join(file)).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("Invalid Grafana JSON in {file}: {e}"));
        let uid = doc["uid"].as_str().unwrap_or("");
        assert!(!uid.is_empty(), "Dashboard {file} missing uid");
        assert!(seen_uids.insert(uid.to_string()), "Duplicate uid '{uid}'");
        let panels = doc["panels"].as_array().unwrap();
        assert!(panels.len() >= 4, "Dashboard {file} has too few panels");
    }
}

#[cfg(test)]
mod live_phase09_analytics_isolation_suite {
    use auth_api::models::EnvironmentId;
    use auth_api::services::analytics::{
        query_environment_analytics, record_environment_api_request, AnalyticsWindow,
    };
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to PostgreSQL 16"]
    async fn test_live_analytics_rollup_strict_environment_isolation() {
        let db_url = match std::env::var("TEST_DATABASE_URL") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => return,
        };
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let suffix = Uuid::new_v4().to_string()[..8].to_string();
        let ws_id: Uuid = sqlx::query_scalar(
            "INSERT INTO developer_workspaces (name, slug) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("P09 WS {suffix}"))
        .bind(format!("p09-ws-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let app_id: Uuid = sqlx::query_scalar(
            "INSERT INTO applications (workspace_id, name, slug) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(ws_id)
        .bind(format!("P09 App {suffix}"))
        .bind(format!("p09-app-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let env_a_id: Uuid = sqlx::query_scalar(
            "INSERT INTO application_environments (application_id, kind, slug) VALUES ($1, 'development', $2) RETURNING id",
        )
        .bind(app_id)
        .bind(format!("p09-dev-env-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let env_b_id: Uuid = sqlx::query_scalar(
            "INSERT INTO application_environments (application_id, kind, slug) VALUES ($1, 'production', $2) RETURNING id",
        )
        .bind(app_id)
        .bind(format!("p09-prod-env-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let env_a = EnvironmentId::from(env_a_id);
        let env_b = EnvironmentId::from(env_b_id);

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, display_name, email_verified) VALUES ($1, 'hash', 'P09 User', TRUE) RETURNING id",
        )
        .bind(format!("p09-user-{suffix}@example.com"))
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO application_users (environment_id, user_id) VALUES ($1, $2)")
            .bind(env_a)
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();

        record_environment_api_request(&pool, env_a, "client").await;
        record_environment_api_request(&pool, env_a, "client").await;
        record_environment_api_request(&pool, env_a, "backend").await;

        let report_a = query_environment_analytics(&pool, env_a, AnalyticsWindow::Days7, true)
            .await
            .unwrap();
        let report_b = query_environment_analytics(&pool, env_b, AnalyticsWindow::Days7, true)
            .await
            .unwrap();

        assert_eq!(report_a.summary.registrations, 1);
        assert_eq!(report_a.summary.client_api_requests, 2);
        assert_eq!(report_a.summary.backend_api_requests, 1);

        assert_eq!(report_b.summary.registrations, 0);
        assert_eq!(report_b.summary.client_api_requests, 0);
        assert_eq!(report_b.summary.backend_api_requests, 0);
        assert_eq!(report_b.summary.dau, 0);
    }
}
