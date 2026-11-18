# Disaster Recovery, Chaos & Incident Runbooks

This document defines verified failure-mode behaviors and step-by-step operational drills for Redis outages, PostgreSQL failover/PITR recovery, webhook worker crashes, graceful shutdowns, and emergency signing-key compromise.

---

## 1. Redis Outage & Graceful Degradation Drill

### Expected Behavior

- **No Pod Panics:** Redis connections use `deadpool_redis::Pool` with non-panicking error paths.
- **Metrics & Alerting:** Redis operation errors increment `auth_redis_errors_total` and fire the `RedisErrorsDetected` alert (`observability/prometheus/alerts.yml`).
- **Degraded Behavior by Subsystem:**
  - Allowed-origins CORS cache (`services/domains.rs`): falls back directly to PostgreSQL `application_domains`.
  - Credential last-used throttle (`services/credentials.rs`): degrades cleanly without blocking authentication.
  - Sessions & refresh tokens: authoritative state lives in PostgreSQL (`app_sessions`, `sso_sessions`, `refresh_tokens`), so verification and revocation continue functioning.
  - Ephemeral lockout/rate-limit counters: fail safely with warnings logged via structured tracing and `/health/ready` surfaces degraded Redis health.

### Drill Procedure

1. Start stack via `docker compose up -d postgres redis auth-api`.
2. Verify baseline `GET /health` and `GET /.well-known/jwks.json` return `200 OK`.
3. Stop Redis: `docker compose stop redis`.
4. Verify `auth-api` remains alive (no panic/restart), `/.well-known/jwks.json` and PostgreSQL-backed APIs remain responsive, and `auth_redis_errors_total` increments in `/metrics`.
5. Restore Redis: `docker compose start redis` and verify automatic pool recovery without restarting `auth-api`.

---

## 2. PostgreSQL Failover & SQLx Pool Recovery Drill

### Expected Behavior

- SQLx `PgPool` drops broken idle connections on reconnect and establishes fresh TCP/TLS sessions once the primary (or promoted replica) accepts connections.
- `PostgresPoolSaturation` alert (`observability/prometheus/alerts.yml`) monitors active pool pressure via `db_pool_connections_in_use`.

### Drill Procedure

1. Run light traffic (`tests/load/soak_test.js` with `SOAK_DURATION=2m`).
2. Restart PostgreSQL (`docker compose restart postgres`) or trigger managed DB failover.
3. Confirm in-flight queries return clean `500`/`503` JSON error envelopes (no stack traces, no process crash loops).
4. Within 5–15 seconds of primary availability, verify `GET /health` and authenticated requests recover automatically with zero manual pod restarts.

---

## 3. Webhook Worker `SIGKILL` Crash & Lease Recovery Drill

### Architecture Guarantee

- `apps/auth-api/src/repositories/webhooks.rs` claims pending outbox rows and due delivery jobs inside a PostgreSQL transaction using `FOR UPDATE SKIP LOCKED`.
- If `webhook-worker` receives `SIGKILL` mid-batch, PostgreSQL automatically rolls back the uncommitted transaction and immediately releases all row locks.
- On restart, the surviving or newly started `webhook-worker` claims the unreleased rows cleanly with zero lost events. Consumers deduplicate at-least-once retries via the stable `X-Auth-Event-Id` header.

### Drill Procedure

1. Enqueue a burst of events (`tests/load/webhook_burst_fanout.js`).
2. Send `SIGKILL` to the worker: `docker compose kill -s KILL webhook-worker`.
3. Verify zero rows are stuck in an inconsistent state in `outbox_events` / `webhook_deliveries`.
4. Restart the worker: `docker compose up -d webhook-worker`.
5. Confirm `auth_outbox_backlog_count` drains to `0` and all events reach `delivered` (or bounded retry/dead-letter for intentionally failing endpoints).

---

## 4. Graceful Shutdown (`SIGTERM`) Drill

### Expected Behavior

- Both `apps/auth-api/src/main.rs` and `apps/auth-api/src/bin/webhook_worker.rs` listen for `SIGTERM` / `SIGINT` and stop accepting new work while allowing in-flight HTTP requests and active database transactions to complete cleanly before process exit (`exit code 0`).

### Drill Procedure

1. Run a steady request stream against `auth-api`.
2. Send `SIGTERM` (`docker compose stop -t 15 auth-api webhook-worker`).
3. Verify zero truncated HTTP responses during drain and exit code `0` in container logs.

---

## 5. PostgreSQL Backup & Point-In-Time Recovery (PITR) Drill

### Critical Requirement

`signing_keys.private_key_encrypted` and `webhook_endpoints.signing_secret_encrypted` are AES-256-GCM envelope-encrypted using `SIGNING_KEY_KEK` + `SIGNING_KEY_KEK_VERSION`. A database restore is only usable when paired with the matching `SIGNING_KEY_KEK` from the secrets manager.

### Drill Procedure

1. **Snapshot:** Create a logical/base backup of PostgreSQL:
   ```bash
   pg_dump --format=custom --no-owner --file=backup_pre_drill.dump "$DATABASE_URL"
   ```
2. **Record Pre-Drill Checksums:** Capture active `signing_keys.kid`, `users` count, `application_users` count, and `app_sessions` count.
3. **Simulate Destructive Incident (in isolated staging DB only):** Drop or corrupt a non-production staging database.
4. **Restore into Clean Database:**
   ```bash
   createdb authdb_restore_drill
   pg_restore --dbname=authdb_restore_drill --no-owner backup_pre_drill.dump
   ```
5. **Verify Cryptographic & Schema Integrity:**
   - Point `auth-api` at `authdb_restore_drill` with the same `SIGNING_KEY_KEK` and `SIGNING_KEY_KEK_VERSION`.
   - Verify `GET /.well-known/jwks.json` returns the exact pre-drill `kid`s.
   - Verify token issuance (confirming `decrypt_signing_key_pkcs8` succeeds) and existing active session validation.
   - Verify webhook secret decryption on a test delivery.

---

## 6. Emergency Signing-Key Compromise Runbook

### Automated Proof

Verified end-to-end against PostgreSQL 16 in `apps/auth-api/tests/security_owasp_asvs_tests.rs` (`test_live_emergency_signing_key_revocation_and_rotation_drill`).

### Incident Steps (Execute in Order)

1. **Rotate First (Ensure Zero Issuance Gap):**
   Generate and activate a fresh ES256 signing keypair (`rotate_signing_key`), transitioning the previous key to `retiring` and activating the new `kid`.
2. **Immediately Revoke Compromised Key:**
   Execute `revoke_signing_key(db, compromised_kid)` (sets `status = 'revoked'`, `revoked_at = NOW()`).
   - `list_jwks_signing_keys` excludes `revoked` keys immediately.
   - `find_verifiable_signing_key_by_kid` rejects `revoked` keys immediately.
   - Downstream resource servers caching `/.well-known/jwks.json` refresh within the JWKS cache window (`Cache-Control` / ETag change).
3. **Force Session Re-Issuance (If Private Key Material Was Exfiltrated):**
   Revoke affected `app_sessions` / `refresh_tokens` if attacker-forged tokens must be invalidated beyond access-token expiry.
4. **Verify Post-Incident State:**
   - Confirm compromised `kid` is absent from `GET /.well-known/jwks.json`.
   - Confirm tokens signed by the compromised `kid` return `401 Unauthorized`.
   - Confirm newly minted tokens use the rotated `kid` and verify cleanly.
   - Inspect `audit_logs` and alert channels to document incident timeline.

