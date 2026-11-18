# Performance Architecture & Load Testing Report

This report documents the concurrency isolation controls, explicit SLO budgets, reproducible k6 benchmark suite (`tests/load/`), and verification procedures for the Dradix Identity & Sync Platform.

---

## 1. Bounded Argon2id Blocking Pool Architecture

Password hashing with Argon2id is intentionally CPU- and memory-bound (~40–120ms per hash depending on hardware). Running Argon2id directly on Tokio async worker threads stalls unrelated lightweight requests (`/.well-known/jwks.json`, `/health`, `/oauth2/userinfo`, `/v1/client/auth/refresh`) under sign-in load spikes.

### Implementation (`apps/auth-api/src/services/password_hasher.rs`)

- **Isolation:** Every password hash and verification across all authentication planes (`client_auth`, `oidc_provider`, `control_auth`, `control_ops`, `authentication`) executes inside `tokio::task::spawn_blocking`.
- **Bounded Concurrency:** A process-wide `tokio::sync::Semaphore` bounds in-flight Argon2id operations to `ARGON2_MAX_CONCURRENCY` (clamped to `1..=32`; defaults to `available_parallelism().clamp(2, 8)`). Excess sign-in requests queue asynchronously without consuming OS threads or starving the async reactor.
- **Timing Parity:** Missing-account sign-in attempts execute `verify_dummy_password_async` through the same bounded pool.
- **Observability:** Each operation records elapsed wall-clock latency to the `argon2_hash_duration_seconds` Prometheus histogram.

### Verification

- Automated unit verification (`test_bounded_argon2id_pool_prevents_async_reactor_starvation` in `apps/auth-api/tests/security_owasp_asvs_tests.rs`) runs 8 concurrent Argon2id verifications on a 2-worker Tokio runtime while asserting a 5ms async timer sentinel never stalls.
- Live load verification (`tests/load/auth_signin_saturation.js`) drives up to 40 req/s Argon2id sign-ins concurrently with 50 req/s `/health` + `/.well-known/jwks.json` probes.

---

## 2. Explicit Performance SLO Budgets

| Plane / Endpoint | Workload Script | p95 Target | p99 Target | Availability / Error Budget |
| --- | --- | --- | --- | --- |
| `GET /.well-known/jwks.json` | `auth_signin_saturation.js`, `token_refresh_and_jwks.js` | `< 15ms` | `< 25ms` | `> 99.9%` |
| `GET /health` | `auth_signin_saturation.js`, `soak_test.js` | `< 15ms` | `< 25ms` | `> 99.9%` |
| `GET /oauth2/userinfo` & discovery | `token_refresh_and_jwks.js` | `< 40ms` | `< 100ms` | `> 99.0%` |
| `POST /v1/client/auth/refresh` | `token_refresh_and_jwks.js` | `< 60ms` | `< 100ms` | Deterministic single-winner race handling |
| `POST /v1/client/auth/sign-in` (under saturation) | `auth_signin_saturation.js` | `< 1500ms` | Bounded by `ARGON2_MAX_CONCURRENCY` | Zero impact on fast-path endpoints |
| `PATCH /v1/backend/users/:id/metadata` (outbox burst) | `webhook_burst_fanout.js` | `< 120ms` | `< 250ms` | `> 99.0%`, zero dropped outbox events |
| `GET /v1/backend/events` (reconciliation catch-up) | `webhook_burst_fanout.js` | `< 80ms` | `< 150ms` | `> 99.0%` |
| Mixed sustained soak (`SOAK_DURATION=2h+`) | `soak_test.js` | `< 15ms` fast / `< 50ms` config | `< 25ms` fast / `< 100ms` config | `< 0.1%` errors, flat RSS & DB pool usage |

---

## 3. Telemetry & Saturation Signals to Monitor During Runs

Inspect the four Grafana dashboards provisioned under `observability/grafana/dashboards/`:

1. **`auth-dashboard.json`:** `argon2_hash_duration_seconds`, `auth_sign_in_duration_seconds`, `auth_http_request_duration_seconds` by plane (`oidc`, `client`, `backend`, `control`, `health`).
2. **`oidc-tokens-dashboard.json`:** `auth_jwks_requests_total`, `auth_refresh_token_reuse_detected_total`, token exchange latency.
3. **`webhooks-sync-dashboard.json`:** `auth_outbox_backlog_count`, `auth_outbox_oldest_age_seconds`, `auth_webhook_deliveries_total`, `auth_webhook_delivery_duration_seconds`, `auth_webhook_dead_letters_total`.
4. **`database-redis-dashboard.json`:** `db_pool_connections_in_use`, `db_pool_connections_idle`, `auth_redis_op_duration_seconds`, `auth_redis_errors_total`.

---

## 4. Environment Execution Checklist (Pre-Production Baseline Sign-Off)

Because CI runners share bursty virtualized CPUs and do not execute multi-hour soak runs on PR commits, execute the following baseline verification in your staging environment before production cutover:

```bash
k6 run tests/load/auth_signin_saturation.js
k6 run tests/load/token_refresh_and_jwks.js
k6 run tests/load/webhook_burst_fanout.js
SOAK_DURATION=2h k6 run tests/load/soak_test.js
```

Record observed p50/p95/p99 latencies, peak `db_pool_connections_in_use`, peak container RSS (start vs end of soak), and `auth_outbox_backlog_count` drain time in the release ticket before Phase 11 production promotion.

