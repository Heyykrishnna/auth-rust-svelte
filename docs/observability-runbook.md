# Observability & Incident Response Runbook

This runbook documents telemetry architecture, end-to-end trace correlation, Prometheus metric contracts, and alert remediation procedures for `auth-api` and `webhook-worker`.

---

## 1. Telemetry Architecture & Endpoints

| Component | Endpoint / Port | Purpose |
| --- | --- | --- |
| `auth-api` Prometheus metrics | `GET http://auth-api:8080/metrics` | Bounded-cardinality domain + plane metrics and DB/outbox/signing-key gauges |
| `webhook-worker` Prometheus metrics | `GET http://webhook-worker:9091/metrics` | Outbound delivery latency, status classes, DLQ, SSRF blocks, and outbox lag |
| OpenTelemetry Collector | `otel-collector:4317` (gRPC OTLP) | Receives spans from `auth-api` and `webhook-worker` and exports to Tempo |
| Grafana Tempo | `http://localhost:3200` | Distributed trace storage across API → Postgres/Redis → Outbox → `webhook-worker` |
| Grafana Loki | `http://localhost:3100` | Structured JSON log aggregation indexed by `plane`, `request_id`, and `trace_id` |
| Prometheus | `http://localhost:9090` | Scrapes metrics every 15s and evaluates `observability/prometheus/alerts.yml` |
| Grafana | `http://localhost:3001` | Auto-provisioned dashboards (`API & Auth Health`, `OIDC & Token Lifecycle`, `Sync & Webhook Pipeline`, `Database & Redis Health`) |

### End-to-End Trace Correlation

1. Every HTTP request receives an `x-request-id` header and starts an OpenTelemetry span (`http_request`) recording `request_id`, `trace_id`, `span_id`, and `plane` (`oidc`, `client`, `backend`, `control`).
2. When a state mutation inserts a row into `outbox_events`, the active W3C `traceparent` is persisted in `outbox_events.traceparent`.
3. When `webhook-worker` claims the delivery, it links its `webhook_delivery` span to that parent context and forwards `traceparent` on the outbound HTTP POST.
4. Search Grafana Tempo by `trace_id` (copied from Loki JSON logs or `x-request-id`) to view the complete timeline from HTTP request to webhook delivery.

### Secret & PII Redaction Guarantee

- Structured JSON logs never include plaintext passwords, OTP codes, raw JWTs (`eyJ...`), secret keys (`sk_live_*`, `sk_test_*`), webhook secrets (`whsec_*`), or invitation tokens (`orginv_*`).
- `redact_sensitive_text` and `redact_sensitive_json` (`apps/auth-api/src/middleware/tracing.rs`) scrub sensitive fields and token prefixes prior to logging.
- Prometheus labels are strictly bounded (`plane`, `method`, `outcome`, `status_class`, `grant_type`, `reason`, `operation`). High-cardinality identifiers (`user_id`, `email`, `environment_id`, `sub`, `ip_address`) are prohibited on Prometheus metrics and computed instead in `analytics_environment_rollups` (PostgreSQL with Row-Level Security).

---

## 2. Alert Playbooks

### High HTTP 5xx Burn Rate

- **Alert:** `HighHttp5xxBurnRate` (`severity: critical`)
- **Trigger:** 5xx ratio > 2% across `http_plane_requests_total` for 3m.
- **Triage:**
  1. Open **API & Auth Health** dashboard to identify which `plane` (`oidc`, `client`, `backend`, `control`) is spiking.
  2. Query Loki for `status >= 500` and copy a `trace_id` into Grafana Tempo.
  3. Check **Database & Redis Health** for SQLx pool saturation or Redis errors.
  4. Verify recent migrations or configuration changes; roll back faulty deploys immediately if correlated.

### Refresh Token Reuse Detected

- **Alert:** `RefreshTokenReuseDetected` (`severity: critical`)
- **Trigger:** `increase(refresh_token_reuse_detected_total[5m]) > 0`
- **Triage:**
  1. Note that `auth-api` has **already automatically revoked the entire refresh token family and associated `app_session`** and emitted `session.revoked`.
  2. Inspect `audit_logs` where `event = 'REFRESH_TOKEN_REUSE_DETECTED'` in the Developer Control Plane to identify affected `environment_id`, IP, and user agent.
  3. Distinguish between benign client concurrency races (missing single-flight refresh lock in a custom client) vs stolen refresh token replay across distinct IPs/ASNs.
  4. If credential theft is suspected, force global session revocation and trigger password reset for the impacted account.

### Brute-Force Lockout Spike

- **Alert:** `AuthBruteForceLockoutSpike` (`severity: warning`)
- **Trigger:** `increase(auth_lockout_total[5m]) >= 10`
- **Triage:**
  1. Check **API & Auth Health** and the Control Plane **Analytics** / **Audit Logs** for `LOGIN_LOCKOUT` and `USER_LOGIN_FAILED` sources.
  2. Verify rate-limiter thresholds (`tower_governor`) and edge WAF rules are actively mitigating credential-stuffing traffic.
  3. Confirm Redis lockout keys are expiring cleanly (`LOGIN_LOCKOUT_DURATION_SECS`).

### Signing Key Nearing Expiry

- **Alert:** `SigningKeyNearingExpiry` (`severity: warning`)
- **Trigger:** `signing_key_min_expiry_seconds < 604800` (less than 7 days remaining).
- **Triage:**
  1. Inspect active and next keys in `signing_keys`.
  2. Provision and promote a new ES256 signing key so JWKS (`/.well-known/jwks.json`) publishes both the retiring and new public keys ahead of rotation.
  3. Verify `SIGNING_KEY_KEK` and `SIGNING_KEY_KEK_VERSION` are synchronized across all `auth-api` replicas.

### Outbox Backlog Age High

- **Alert:** `OutboxBacklogAgeHigh` (`severity: critical`) / `OutboxPendingDepthHigh` (`severity: warning`)
- **Trigger:** `outbox_oldest_age_seconds > 60` for 2m or `outbox_pending_depth > 500` for 3m.
- **Triage:**
  1. Check `webhook-worker` liveness (`GET http://webhook-worker:9091/health`) and container logs.
  2. Open **Sync & Webhook Pipeline** dashboard to determine if outbound delivery latency spiked or if workers stalled.
  3. Scale `webhook-worker` replicas or raise `WEBHOOK_WORKER_BATCH_SIZE` (`FOR UPDATE SKIP LOCKED` guarantees safe horizontal concurrency).

### Webhook Dead-Letter Surge

- **Alert:** `WebhookDeadLetterSurge` (`severity: warning`)
- **Trigger:** `increase(webhook_dead_letter_total[10m]) >= 5`
- **Triage:**
  1. Open Control Plane **Webhooks & Sync** for affected environments to inspect HTTP response status codes and response body excerpts.
  2. Confirm whether the downstream endpoint returned permanent 4xx errors or suffered prolonged downtime (which triggers the circuit breaker).
  3. Once the consumer fixes their endpoint, resume paused endpoints and run time-range replay (`POST .../webhooks/:endpoint_id/replay`) or have the consumer reconcile via `GET /v1/backend/users?updated_since=...` and `GET /v1/backend/organizations?updated_since=...`.

### Webhook SSRF Blocked

- **Alert:** `WebhookSsrfBlockedSpike` (`severity: warning`)
- **Trigger:** `increase(webhook_ssrf_blocked_total[5m]) >= 3`
- **Triage:**
  1. Inspect `webhook_deliveries` response excerpts for DNS rebinding, private IP resolution, or HTTP 3xx redirect attempts.
  2. Audit the developer who configured or mutated the offending `webhook_endpoints` URL in `audit_logs`.
  3. Disable malicious or misconfigured endpoints.

### Postgres Pool Saturation

- **Alert:** `PostgresPoolSaturation` (`severity: critical`)
- **Trigger:** Active SQLx connections > 85% of pool capacity for 3m.
- **Triage:**
  1. Check PostgreSQL `pg_stat_activity` for long-running queries or lock contention.
  2. Verify `analytics_environment_rollups` indexes are intact and no ad-hoc full-table scans are running on `audit_logs`.
  3. Tune `DATABASE_MAX_CONNECTIONS` or introduce PgBouncer transaction pooling if load has legitimately grown.

### Redis Unreachable

- **Alert:** `RedisErrorsDetected` (`severity: critical`)
- **Trigger:** `increase(redis_errors_total[5m]) >= 5`
- **Triage:**
  1. Verify Redis container/instance health, memory pressure, and eviction metrics.
  2. Confirm `REDIS_URL` credentials and network reachability from `auth-api`.
  3. Note that OTP verification, login lockout counters, and access-token blacklists depend on Redis; restore Redis before clearing incident status.

