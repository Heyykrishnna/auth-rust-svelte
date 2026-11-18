# Load, Stress & Soak Testing Suite (`tests/load/`)

Reproducible [k6](https://k6.io/) workloads enforcing the Dradix Identity & Sync Platform SLO budgets before production rollout.

## Performance SLO Budgets

| Workload | Metric | Budget |
| --- | --- | --- |
| Stateless JWKS (`/.well-known/jwks.json`) & `/health` | p95 / p99 | `< 15ms` / `< 25ms` |
| OIDC Discovery & `/oauth2/userinfo` | p95 / p99 | `< 40ms` / `< 100ms` |
| Token Refresh (`/v1/client/auth/refresh`) | p95 / p99 | `< 60ms` / `< 100ms` |
| Argon2id Sign-In Saturation (`/v1/client/auth/sign-in`) | Fast-path availability | `> 99.9%` with JWKS/health p99 `< 25ms` |
| Backend Sync & Outbox Burst (`/v1/backend/*`) | p95 / p99 | `< 120ms` / `< 250ms` |
| Multi-Hour Soak (`soak_test.js`) | Error rate & leaks | `< 0.1%` HTTP errors, zero DB connection or RSS growth |

## Prerequisites

1. Start the local or staging stack (`docker compose up -d postgres redis auth-api webhook-worker prometheus`).
2. Provision a test application environment and capture:
   - `AUTH_API_URL` (default `http://127.0.0.1:8080`)
   - `PUBLISHABLE_KEY` (`pk_test_...`)
   - `BACKEND_SECRET_KEY` (`sk_test_...`, scoped with `users:read users:write sync:read`)
   - `TARGET_APP_USER_ID` (UUID of a seeded `application_users` row for webhook fan-out tests)
3. Install `k6` (`brew install k6` on macOS or use `grafana/k6` Docker image).

## Workloads

### 1. Argon2id Sign-In Saturation & Non-Starvation Proof
```bash
AUTH_API_URL=http://127.0.0.1:8080 \
PUBLISHABLE_KEY=pk_test_... \
LOAD_TEST_EMAIL=load@example.com \
LOAD_TEST_PASSWORD='ChangeMeStrong!2026' \
k6 run tests/load/auth_signin_saturation.js
```
Verifies that the bounded `ARGON2_MAX_CONCURRENCY` blocking pool (`apps/auth-api/src/services/password_hasher.rs`) absorbs concurrent password hashing spikes without stalling Tokio worker threads serving `/health` and `/.well-known/jwks.json`.

### 2. Token Refresh Races, JWKS & Userinfo Read Spikes
```bash
AUTH_API_URL=http://127.0.0.1:8080 \
PUBLISHABLE_KEY=pk_test_... \
LOAD_ACCESS_TOKEN=... \
LOAD_REFRESH_TOKEN=... \
k6 run tests/load/token_refresh_and_jwks.js
```
Exercises 400 req/s JWKS reads, 100 req/s `/oauth2/userinfo` reads, and simultaneous refresh token replay across 4 VUs to verify deterministic refresh-family handling.

### 3. Burst Webhook Fan-Out & Event Reconciliation
```bash
AUTH_API_URL=http://127.0.0.1:8080 \
BACKEND_SECRET_KEY=sk_test_... \
TARGET_APP_USER_ID=... \
k6 run tests/load/webhook_burst_fanout.js
```
Generates sustained `user.updated` outbox bursts while running concurrent `/v1/backend/events` catch-up reads. Monitor `auth_outbox_backlog_count`, `auth_outbox_oldest_age_seconds`, and `auth_webhook_deliveries_total` in Grafana (`webhooks-sync-dashboard.json`).

### 4. Multi-Hour Soak Test
```bash
AUTH_API_URL=http://127.0.0.1:8080 \
SOAK_DURATION=2h \
PUBLISHABLE_KEY=pk_test_... \
BACKEND_SECRET_KEY=sk_test_... \
k6 run tests/load/soak_test.js
```
Verifies stable RSS memory, zero leaked PostgreSQL pool connections (`db_pool_connections_in_use`), and flat p99 latency over extended durations.

