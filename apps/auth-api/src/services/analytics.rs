use std::collections::HashMap;

use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::errors::AppError;
use crate::models::{ApplicationId, DeveloperId, EnvironmentId, EnvironmentKind, WorkspaceRole};
use crate::services::control_plane;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsWindow {
    Hours24,
    Days7,
    Days30,
    Days90,
}

impl AnalyticsWindow {
    pub fn parse(raw: Option<&str>) -> Result<Self, AppError> {
        match raw.map(str::trim).unwrap_or("7d").to_lowercase().as_str() {
            "24h" => Ok(Self::Hours24),
            "7d" => Ok(Self::Days7),
            "30d" => Ok(Self::Days30),
            "90d" => Ok(Self::Days90),
            other => Err(AppError::Validation(format!(
                "Invalid analytics window '{other}'; expected 24h, 7d, 30d, or 90d"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hours24 => "24h",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::Days90 => "90d",
        }
    }

    pub fn granularity(self) -> &'static str {
        match self {
            Self::Hours24 => "hour",
            Self::Days7 | Self::Days30 | Self::Days90 => "day",
        }
    }

    pub fn bucket_count(self) -> i64 {
        match self {
            Self::Hours24 => 24,
            Self::Days7 => 7,
            Self::Days30 => 30,
            Self::Days90 => 90,
        }
    }

    pub fn truncate_bucket(self, ts: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Hours24 => ts
                .with_minute(0)
                .and_then(|t| t.with_second(0))
                .and_then(|t| t.with_nanosecond(0))
                .unwrap_or(ts),
            Self::Days7 | Self::Days30 | Self::Days90 => ts
                .with_hour(0)
                .and_then(|t| t.with_minute(0))
                .and_then(|t| t.with_second(0))
                .and_then(|t| t.with_nanosecond(0))
                .unwrap_or(ts),
        }
    }

    pub fn step(self) -> Duration {
        match self {
            Self::Hours24 => Duration::hours(1),
            Self::Days7 | Self::Days30 | Self::Days90 => Duration::days(1),
        }
    }

    pub fn window_start(self, now: DateTime<Utc>) -> DateTime<Utc> {
        let current = self.truncate_bucket(now);
        current - self.step() * (self.bucket_count() - 1) as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReasonCount {
    pub reason: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderBreakdown {
    pub password: i64,
    pub google: i64,
    pub github: i64,
    pub oidc_sso: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub registrations: i64,
    pub dau: i64,
    pub mau: i64,
    pub sign_in_success: i64,
    pub sign_in_failure: i64,
    pub lockouts: i64,
    pub active_sessions: i64,
    pub sessions_created: i64,
    pub sessions_revoked: i64,
    pub refresh_reuse_detected: i64,
    pub client_api_requests: i64,
    pub backend_api_requests: i64,
    pub webhook_deliveries_succeeded: i64,
    pub webhook_deliveries_failed: i64,
    pub webhook_deliveries_dead_letter: i64,
    pub webhook_success_rate: f64,
    pub webhook_avg_latency_ms: f64,
    pub failure_reasons: Vec<FailureReasonCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSeriesPoint {
    pub bucket_start: DateTime<Utc>,
    pub registrations: i64,
    pub active_users: i64,
    pub sign_in_success: i64,
    pub sign_in_failure: i64,
    pub client_api_requests: i64,
    pub backend_api_requests: i64,
    pub webhook_succeeded: i64,
    pub webhook_failed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentAnalyticsResponse {
    pub environment_id: EnvironmentId,
    pub window: String,
    pub granularity: String,
    pub generated_at: DateTime<Utc>,
    pub summary: AnalyticsSummary,
    pub provider_breakdown: ProviderBreakdown,
    pub series: Vec<AnalyticsSeriesPoint>,
}

#[derive(Debug, FromRow)]
struct RollupRow {
    bucket_start: DateTime<Utc>,
    registrations_count: i64,
    active_users_count: i64,
    sign_ins_success: i64,
    sign_ins_failure: i64,
    sign_ins_password: i64,
    sign_ins_google: i64,
    sign_ins_github: i64,
    sign_ins_oidc_sso: i64,
    lockouts_count: i64,
    sessions_created: i64,
    sessions_revoked: i64,
    refresh_reuse_detected: i64,
    client_api_requests: i64,
    backend_api_requests: i64,
    webhook_deliveries_succeeded: i64,
    webhook_deliveries_failed: i64,
    webhook_deliveries_dead_letter: i64,
    webhook_latency_sum_ms: i64,
    webhook_latency_count: i64,
}

#[derive(Debug, FromRow)]
struct ReasonRow {
    reason: String,
    count: i64,
}

pub fn compute_webhook_success_rate(succeeded: i64, failed: i64, dead_letter: i64) -> f64 {
    let total = succeeded + failed + dead_letter;
    if total <= 0 {
        return 100.0;
    }
    ((succeeded as f64 / total as f64) * 10_000.0).round() / 100.0
}

pub async fn record_environment_api_request(
    db: &PgPool,
    environment_id: EnvironmentId,
    plane: &str,
) {
    let is_client = plane == "client";
    let is_backend = plane == "backend";
    if !is_client && !is_backend {
        return;
    }

    let now = Utc::now();
    let hour_bucket = AnalyticsWindow::Hours24.truncate_bucket(now);
    let day_bucket = AnalyticsWindow::Days7.truncate_bucket(now);
    let client_inc: i64 = if is_client { 1 } else { 0 };
    let backend_inc: i64 = if is_backend { 1 } else { 0 };

    for (granularity, bucket) in [("hour", hour_bucket), ("day", day_bucket)] {
        let _ = sqlx::query(
            r#"
            INSERT INTO analytics_environment_rollups (
                environment_id, bucket_granularity, bucket_start,
                client_api_requests, backend_api_requests, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (environment_id, bucket_granularity, bucket_start)
            DO UPDATE SET
                client_api_requests = analytics_environment_rollups.client_api_requests + EXCLUDED.client_api_requests,
                backend_api_requests = analytics_environment_rollups.backend_api_requests + EXCLUDED.backend_api_requests,
                updated_at = NOW()
            "#,
        )
        .bind(environment_id)
        .bind(granularity)
        .bind(bucket)
        .bind(client_inc)
        .bind(backend_inc)
        .execute(db)
        .await;
    }
}

pub async fn refresh_environment_rollups(
    db: &PgPool,
    environment_id: EnvironmentId,
    window: AnalyticsWindow,
) -> Result<usize, AppError> {
    let granularity = window.granularity();
    let since = window.window_start(Utc::now());

    let result = sqlx::query(
        r#"
        WITH buckets AS (
            SELECT date_trunc($2, created_at) AS bucket_start
            FROM application_users
            WHERE environment_id = $1 AND created_at >= $3
            UNION
            SELECT date_trunc($2, created_at) AS bucket_start
            FROM audit_logs
            WHERE environment_id = $1 AND created_at >= $3
            UNION
            SELECT date_trunc($2, created_at) AS bucket_start
            FROM app_sessions
            WHERE environment_id = $1 AND created_at >= $3
            UNION
            SELECT date_trunc($2, created_at) AS bucket_start
            FROM webhook_deliveries
            WHERE environment_id = $1 AND created_at >= $3
        ),
        regs AS (
            SELECT date_trunc($2, created_at) AS bucket_start, COUNT(*)::bigint AS registrations_count
            FROM application_users
            WHERE environment_id = $1 AND created_at >= $3
            GROUP BY 1
        ),
        active_sess AS (
            SELECT
                date_trunc($2, created_at) AS bucket_start,
                COUNT(DISTINCT application_user_id)::bigint AS active_users_count,
                COUNT(*)::bigint AS sessions_created,
                COUNT(*) FILTER (WHERE revoked_at IS NOT NULL)::bigint AS sessions_revoked
            FROM app_sessions
            WHERE environment_id = $1 AND created_at >= $3
            GROUP BY 1
        ),
        audits AS (
            SELECT
                date_trunc($2, created_at) AS bucket_start,
                COUNT(*) FILTER (
                    WHERE event IN ('USER_LOGIN_SUCCESS', 'CLIENT_SIGN_IN', 'SOCIAL_OAUTH_SIGN_IN', 'OIDC_AUTHORIZE_SSO_HIT', 'OIDC_PORTAL_LOGIN')
                )::bigint AS sign_ins_success,
                COUNT(*) FILTER (
                    WHERE event IN ('USER_LOGIN_FAILED', 'CLIENT_SIGN_IN_FAILED', 'LOGIN_LOCKOUT', 'SOCIAL_OAUTH_FAILED')
                )::bigint AS sign_ins_failure,
                COUNT(*) FILTER (
                    WHERE event IN ('USER_LOGIN_SUCCESS', 'CLIENT_SIGN_IN', 'OIDC_PORTAL_LOGIN')
                      AND COALESCE(metadata->>'provider', 'password') = 'password'
                )::bigint AS sign_ins_password,
                COUNT(*) FILTER (
                    WHERE LOWER(COALESCE(metadata->>'provider', '')) = 'google'
                )::bigint AS sign_ins_google,
                COUNT(*) FILTER (
                    WHERE LOWER(COALESCE(metadata->>'provider', '')) = 'github'
                )::bigint AS sign_ins_github,
                COUNT(*) FILTER (
                    WHERE event IN ('OIDC_AUTHORIZE_SSO_HIT', 'OIDC_SSO_SILENT_HIT')
                )::bigint AS sign_ins_oidc_sso,
                COUNT(*) FILTER (WHERE event = 'LOGIN_LOCKOUT')::bigint AS lockouts_count,
                COUNT(*) FILTER (WHERE event = 'REFRESH_TOKEN_REUSE_DETECTED')::bigint AS refresh_reuse_detected
            FROM audit_logs
            WHERE environment_id = $1 AND created_at >= $3
            GROUP BY 1
        ),
        webhooks AS (
            SELECT
                date_trunc($2, created_at) AS bucket_start,
                COUNT(*) FILTER (WHERE status = 'succeeded')::bigint AS webhook_deliveries_succeeded,
                COUNT(*) FILTER (WHERE status IN ('failed', 'retrying'))::bigint AS webhook_deliveries_failed,
                COUNT(*) FILTER (WHERE status = 'dead_letter')::bigint AS webhook_deliveries_dead_letter,
                COALESCE(SUM(COALESCE(latency_ms, 0)) FILTER (WHERE latency_ms IS NOT NULL), 0)::bigint AS webhook_latency_sum_ms,
                COUNT(*) FILTER (WHERE latency_ms IS NOT NULL)::bigint AS webhook_latency_count
            FROM webhook_deliveries
            WHERE environment_id = $1 AND created_at >= $3
            GROUP BY 1
        )
        INSERT INTO analytics_environment_rollups (
            environment_id,
            bucket_granularity,
            bucket_start,
            registrations_count,
            active_users_count,
            sign_ins_success,
            sign_ins_failure,
            sign_ins_password,
            sign_ins_google,
            sign_ins_github,
            sign_ins_oidc_sso,
            lockouts_count,
            sessions_created,
            sessions_revoked,
            refresh_reuse_detected,
            webhook_deliveries_succeeded,
            webhook_deliveries_failed,
            webhook_deliveries_dead_letter,
            webhook_latency_sum_ms,
            webhook_latency_count,
            updated_at
        )
        SELECT
            $1,
            $2,
            b.bucket_start,
            COALESCE(r.registrations_count, 0),
            COALESCE(s.active_users_count, 0),
            COALESCE(a.sign_ins_success, 0),
            COALESCE(a.sign_ins_failure, 0),
            COALESCE(a.sign_ins_password, 0),
            COALESCE(a.sign_ins_google, 0),
            COALESCE(a.sign_ins_github, 0),
            COALESCE(a.sign_ins_oidc_sso, 0),
            COALESCE(a.lockouts_count, 0),
            COALESCE(s.sessions_created, 0),
            COALESCE(s.sessions_revoked, 0),
            COALESCE(a.refresh_reuse_detected, 0),
            COALESCE(w.webhook_deliveries_succeeded, 0),
            COALESCE(w.webhook_deliveries_failed, 0),
            COALESCE(w.webhook_deliveries_dead_letter, 0),
            COALESCE(w.webhook_latency_sum_ms, 0),
            COALESCE(w.webhook_latency_count, 0),
            NOW()
        FROM buckets b
        LEFT JOIN regs r ON r.bucket_start = b.bucket_start
        LEFT JOIN active_sess s ON s.bucket_start = b.bucket_start
        LEFT JOIN audits a ON a.bucket_start = b.bucket_start
        LEFT JOIN webhooks w ON w.bucket_start = b.bucket_start
        ON CONFLICT (environment_id, bucket_granularity, bucket_start)
        DO UPDATE SET
            registrations_count = EXCLUDED.registrations_count,
            active_users_count = EXCLUDED.active_users_count,
            sign_ins_success = EXCLUDED.sign_ins_success,
            sign_ins_failure = EXCLUDED.sign_ins_failure,
            sign_ins_password = EXCLUDED.sign_ins_password,
            sign_ins_google = EXCLUDED.sign_ins_google,
            sign_ins_github = EXCLUDED.sign_ins_github,
            sign_ins_oidc_sso = EXCLUDED.sign_ins_oidc_sso,
            lockouts_count = EXCLUDED.lockouts_count,
            sessions_created = EXCLUDED.sessions_created,
            sessions_revoked = EXCLUDED.sessions_revoked,
            refresh_reuse_detected = EXCLUDED.refresh_reuse_detected,
            webhook_deliveries_succeeded = EXCLUDED.webhook_deliveries_succeeded,
            webhook_deliveries_failed = EXCLUDED.webhook_deliveries_failed,
            webhook_deliveries_dead_letter = EXCLUDED.webhook_deliveries_dead_letter,
            webhook_latency_sum_ms = EXCLUDED.webhook_latency_sum_ms,
            webhook_latency_count = EXCLUDED.webhook_latency_count,
            updated_at = NOW()
        "#,
    )
    .bind(environment_id)
    .bind(granularity)
    .bind(since)
    .execute(db)
    .await?;

    Ok(result.rows_affected() as usize)
}

pub async fn query_environment_analytics(
    db: &PgPool,
    environment_id: EnvironmentId,
    window: AnalyticsWindow,
    refresh_first: bool,
) -> Result<EnvironmentAnalyticsResponse, AppError> {
    if refresh_first {
        refresh_environment_rollups(db, environment_id, window).await?;
    }

    let now = Utc::now();
    let since = window.window_start(now);
    let granularity = window.granularity();

    let rows = sqlx::query_as::<_, RollupRow>(
        r#"
        SELECT
            bucket_start,
            registrations_count,
            active_users_count,
            sign_ins_success,
            sign_ins_failure,
            sign_ins_password,
            sign_ins_google,
            sign_ins_github,
            sign_ins_oidc_sso,
            lockouts_count,
            sessions_created,
            sessions_revoked,
            refresh_reuse_detected,
            client_api_requests,
            backend_api_requests,
            webhook_deliveries_succeeded,
            webhook_deliveries_failed,
            webhook_deliveries_dead_letter,
            webhook_latency_sum_ms,
            webhook_latency_count
        FROM analytics_environment_rollups
        WHERE environment_id = $1
          AND bucket_granularity = $2
          AND bucket_start >= $3
        ORDER BY bucket_start ASC
        "#,
    )
    .bind(environment_id)
    .bind(granularity)
    .bind(since)
    .fetch_all(db)
    .await?;

    let (dau,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT application_user_id)::bigint
        FROM app_sessions
        WHERE environment_id = $1 AND last_used_at >= NOW() - INTERVAL '24 hours'
        "#,
    )
    .bind(environment_id)
    .fetch_one(db)
    .await?;

    let (mau,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT application_user_id)::bigint
        FROM app_sessions
        WHERE environment_id = $1 AND last_used_at >= NOW() - INTERVAL '30 days'
        "#,
    )
    .bind(environment_id)
    .fetch_one(db)
    .await?;

    let (active_sessions,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM app_sessions
        WHERE environment_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
        "#,
    )
    .bind(environment_id)
    .fetch_one(db)
    .await?;

    let failure_rows = sqlx::query_as::<_, ReasonRow>(
        r#"
        SELECT
            COALESCE(NULLIF(metadata->>'reason', ''), event) AS reason,
            COUNT(*)::bigint AS count
        FROM audit_logs
        WHERE environment_id = $1
          AND created_at >= $2
          AND event IN ('USER_LOGIN_FAILED', 'CLIENT_SIGN_IN_FAILED', 'LOGIN_LOCKOUT', 'SOCIAL_OAUTH_FAILED')
        GROUP BY 1
        ORDER BY count DESC
        LIMIT 8
        "#,
    )
    .bind(environment_id)
    .bind(since)
    .fetch_all(db)
    .await?;

    let mut by_bucket: HashMap<DateTime<Utc>, &RollupRow> = HashMap::new();
    let mut registrations = 0i64;
    let mut sign_in_success = 0i64;
    let mut sign_in_failure = 0i64;
    let mut sign_ins_password = 0i64;
    let mut sign_ins_google = 0i64;
    let mut sign_ins_github = 0i64;
    let mut sign_ins_oidc_sso = 0i64;
    let mut lockouts = 0i64;
    let mut sessions_created = 0i64;
    let mut sessions_revoked = 0i64;
    let mut refresh_reuse_detected = 0i64;
    let mut client_api_requests = 0i64;
    let mut backend_api_requests = 0i64;
    let mut webhook_succeeded = 0i64;
    let mut webhook_failed = 0i64;
    let mut webhook_dead_letter = 0i64;
    let mut webhook_latency_sum = 0i64;
    let mut webhook_latency_count = 0i64;

    for row in &rows {
        by_bucket.insert(row.bucket_start, row);
        registrations += row.registrations_count;
        sign_in_success += row.sign_ins_success;
        sign_in_failure += row.sign_ins_failure;
        sign_ins_password += row.sign_ins_password;
        sign_ins_google += row.sign_ins_google;
        sign_ins_github += row.sign_ins_github;
        sign_ins_oidc_sso += row.sign_ins_oidc_sso;
        lockouts += row.lockouts_count;
        sessions_created += row.sessions_created;
        sessions_revoked += row.sessions_revoked;
        refresh_reuse_detected += row.refresh_reuse_detected;
        client_api_requests += row.client_api_requests;
        backend_api_requests += row.backend_api_requests;
        webhook_succeeded += row.webhook_deliveries_succeeded;
        webhook_failed += row.webhook_deliveries_failed;
        webhook_dead_letter += row.webhook_deliveries_dead_letter;
        webhook_latency_sum += row.webhook_latency_sum_ms;
        webhook_latency_count += row.webhook_latency_count;
    }

    let step = window.step();
    let mut series = Vec::with_capacity(window.bucket_count() as usize);
    for idx in 0..window.bucket_count() {
        let bucket = since + step * idx as i32;
        if let Some(r) = by_bucket.get(&bucket) {
            series.push(AnalyticsSeriesPoint {
                bucket_start: bucket,
                registrations: r.registrations_count,
                active_users: r.active_users_count,
                sign_in_success: r.sign_ins_success,
                sign_in_failure: r.sign_ins_failure,
                client_api_requests: r.client_api_requests,
                backend_api_requests: r.backend_api_requests,
                webhook_succeeded: r.webhook_deliveries_succeeded,
                webhook_failed: r.webhook_deliveries_failed + r.webhook_deliveries_dead_letter,
            });
        } else {
            series.push(AnalyticsSeriesPoint {
                bucket_start: bucket,
                registrations: 0,
                active_users: 0,
                sign_in_success: 0,
                sign_in_failure: 0,
                client_api_requests: 0,
                backend_api_requests: 0,
                webhook_succeeded: 0,
                webhook_failed: 0,
            });
        }
    }

    let webhook_avg_latency_ms = if webhook_latency_count > 0 {
        ((webhook_latency_sum as f64 / webhook_latency_count as f64) * 100.0).round() / 100.0
    } else {
        0.0
    };

    Ok(EnvironmentAnalyticsResponse {
        environment_id,
        window: window.as_str().to_string(),
        granularity: granularity.to_string(),
        generated_at: now,
        summary: AnalyticsSummary {
            registrations,
            dau,
            mau,
            sign_in_success,
            sign_in_failure,
            lockouts,
            active_sessions,
            sessions_created,
            sessions_revoked,
            refresh_reuse_detected,
            client_api_requests,
            backend_api_requests,
            webhook_deliveries_succeeded: webhook_succeeded,
            webhook_deliveries_failed: webhook_failed,
            webhook_deliveries_dead_letter: webhook_dead_letter,
            webhook_success_rate: compute_webhook_success_rate(
                webhook_succeeded,
                webhook_failed,
                webhook_dead_letter,
            ),
            webhook_avg_latency_ms,
            failure_reasons: failure_rows
                .into_iter()
                .map(|r| FailureReasonCount {
                    reason: r.reason,
                    count: r.count,
                })
                .collect(),
        },
        provider_breakdown: ProviderBreakdown {
            password: sign_ins_password,
            google: sign_ins_google,
            github: sign_ins_github,
            oidc_sso: sign_ins_oidc_sso,
        },
        series,
    })
}

pub async fn get_environment_analytics_for_developer(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    window: AnalyticsWindow,
) -> Result<EnvironmentAnalyticsResponse, AppError> {
    let env = control_plane::get_environment_for_developer(
        db,
        developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    query_environment_analytics(db, env.id, window, true).await
}

pub async fn trigger_environment_rollup_refresh_for_developer(
    db: &PgPool,
    developer_id: DeveloperId,
    app_id: ApplicationId,
    kind: EnvironmentKind,
    window: AnalyticsWindow,
) -> Result<usize, AppError> {
    let env = control_plane::get_environment_for_developer(
        db,
        developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    refresh_environment_rollups(db, env.id, window).await
}
