# Phase 09: Observability, Per-App Analytics & Operational Intelligence

- **Status:** Planned
- **Prerequisites:** Phase 05 (Workers/Outbox) & Phase 07 (Developer Dashboard) completed
- **Primary Targets:** `apps/auth-api/src/middleware/tracing.rs`, `apps/auth-api/src/services/analytics.rs`, `observability/`, `apps/web/src/routes/dashboard/analytics/`

---

## Objective

Extend the repository's existing OpenTelemetry, Prometheus, Grafana, Loki, and Tempo stack (`observability/`) into full multi-tenant operational observability—giving platform engineers real-time incident visibility and giving application developers accurate, strictly isolated usage and authentication analytics in the Dashboard.

---

## 1. Backend & Worker Telemetry (`apps/auth-api` & `webhook_worker`)

- **Structured JSON Logging & Correlation:**
  - Attach `request_id`, OpenTelemetry `trace_id` / `span_id`, `plane` (`oidc`, `client`, `backend`, `control`), and `environment_id` (bounded cardinality; never log raw passwords, OTP codes, JWTs, `sk_*`, or `whsec_*` secrets).
- **Distributed Tracing (OTLP -> Tempo):**
  - Trace spans across HTTP handlers -> PostgreSQL queries -> Redis ops -> upstream OAuth provider HTTP calls -> outbox enqueue -> `webhook_worker` outbound HTTP deliveries (propagating trace context cleanly via outbox metadata).
- **Domain & Infrastructure Metrics (Prometheus):**
  - **Authentication:** `auth_sign_in_total{method,outcome}`, `auth_sign_up_total{outcome}`, `auth_lockout_total`, `auth_latency_seconds`.
  - **OIDC & SSO:** `oidc_authorize_total{policy_outcome}`, `oidc_sso_hit_total`, `oidc_token_exchange_total{grant_type,outcome}`, `jwks_requests_total`.
  - **Tokens & Sessions:** `session_created_total`, `session_revoked_total{reason}`, `refresh_token_reuse_detected_total` (high-severity alert metric).
  - **Sync & Webhooks:** `outbox_pending_depth`, `outbox_oldest_age_seconds`, `webhook_delivery_total{status_class}`, `webhook_delivery_latency_seconds`, `webhook_dead_letter_total`, `webhook_ssrf_blocked_total`.
  - **Data Plane:** SQLx pool active/idle/wait time, Redis latency/error rate, Argon2id CPU queue duration.
  - **Cardinality Guardrail:** Never attach unbounded `user_id`, `email`, or arbitrary `environment_id` directly onto high-cardinality Prometheus metric labels; keep Prometheus labels bounded and compute per-app analytics in PostgreSQL rollup tables.

---

## 2. Grafana Dashboards & Alerting (`observability/grafana/`, `observability/prometheus/`)

Expand `observability/grafana/dashboards/auth-dashboard.json` into dedicated operational dashboards + alert rules:

- **API & Auth Health Dashboard:** Request rate, p50/p95/p99 latency per plane, 4xx/5xx error budgets, Argon2id latency, brute-force lockout spikes.
- **OIDC & Token Lifecycle Dashboard:** SSO hit ratio vs interactive logins, token exchange latency, refresh reuse detections, signing key expiration countdown.
- **Sync & Webhook Pipeline Dashboard:** Outbox queue depth, oldest unprocessed event age, delivery success rate, retry volume, dead-letter queue growth, worker liveness.
- **Database & Redis Health Dashboard:** Postgres connection pool saturation, slow queries, replication/checkpoint lag, Redis memory/evictions.
- **Actionable Alert Rules:** High 5xx burn rate, `refresh_token_reuse_detected_total` spike, `outbox_oldest_age_seconds > 60s`, `webhook_dead_letter_total` surge, signing key nearing expiry, DB pool exhaustion, Redis unreachable.

---

## 3. Per-Environment Application Analytics (Developer-Facing)

- **Rollup Pipeline:** Maintain hourly/daily PostgreSQL rollup tables (`analytics_environment_rollups`) populated asynchronously (or via scheduled SQL rollups) so Dashboard analytics queries remain fast without scanning raw `audit_logs`.
- **Metrics Exposed Strictly per `environment_id`:**
  - New user registrations, Daily Active Users (DAU) & Monthly Active Users (MAU).
  - Sign-in success vs. failure counts & failure reasons.
  - Provider breakdown (password, Google, GitHub, OIDC SSO silent hit).
  - Active session counts, API request volume (`client` vs `backend`), and webhook delivery success/latency rates.
- **Developer Analytics Dashboard (`apps/web`):**
  - Date-range picker (24h, 7d, 30d, 90d), environment switcher integration, interactive charts, auth method breakdown, webhook reliability summary, and drill-down links to Audit & Delivery Logs.

---

## Required Tests & Verification Gates (Executed in Phase 09)

- **PII / Secret Redaction Log Test:** Trigger auth failures, secret-key auth, webhook deliveries, and password resets; inspect captured JSON logs to assert zero plaintext passwords, tokens, OTPs, `sk_*`, or `whsec_*` appear.
- **Analytics Isolation Test:** Verify rollup queries for Environment A never include registrations, DAU/MAU, or API counts from Environment B.
- **Metrics & Alert Syntax Validation:** Verify `/metrics` exposes all documented metrics without unbounded label explosion; validate Prometheus rule files (`promtool check rules`) and Grafana dashboard JSON provisioning.

---

## Deliverables

- Multi-tenant OTel tracing, redacted JSON logging, and Prometheus instrumentation in `apps/auth-api` and `webhook_worker`
- Updated Grafana dashboards & Prometheus alert rules under `observability/`
- PostgreSQL analytics rollup schema, job, and `/v1/control/.../analytics` API
- Developer Analytics Dashboard views in `apps/web`
- Incident Monitoring & Alert Playbook (`docs/observability-runbook.md`)

---

## Acceptance Criteria

- [ ] A single request can be traced end-to-end across HTTP API -> Postgres/Redis -> Outbox -> `webhook_worker` in Grafana Tempo using its correlation/trace ID.
- [ ] Zero sensitive credentials, passwords, or raw tokens appear in structured logs or trace attributes.
- [ ] Operational alerts fire accurately under simulated outbox backlog, high 5xx rate, and refresh token reuse anomalies.
- [ ] Developers see accurate, environment-isolated DAU/MAU, auth method breakdown, and webhook reliability charts in the Dashboard.

---

## Out of Scope

- Automated billing invoice generation (Phase 12 consumes these metering/analytics primitives)

