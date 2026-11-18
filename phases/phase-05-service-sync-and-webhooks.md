# Phase 05: Service Synchronization, Webhooks & Backend Reconciliation API

- **Status:** Planned
- **Prerequisites:** Phase 03 & Phase 04 completed (`outbox_events`, `webhook_endpoints`, `webhook_deliveries`, `SecretKeyAuth`, monotonic entity `version`)
- **Primary Targets:** `apps/auth-api/src/services/events.rs`, `apps/auth-api/src/workers/webhooks.rs` (or separate worker binary in `apps/auth-api/src/bin/webhook_worker.rs`), `apps/auth-api/src/routes/backend.rs`, `apps/auth-api/src/routes/control_webhooks.rs`

---

## Objective

Directly fulfill the core requirement that **any external project (Ferrum, Elysiar, or any future app) plugging into this platform stays reliably, deterministically, and securely synchronized with identity, membership, session, and organization state**—handling real-time delivery, out-of-order protection, downtime recovery, and full backfill reconciliation.

---

## 1. Versioned Event Schema & Transactional Outbox

- **Transactional Outbox Emission:**
  - Every state-changing domain transaction writes an `outbox_events` row atomically alongside the primary mutation (never fire-and-forget `tokio::spawn` for webhooks).
  - Canonical event envelope (`v1`):
    ```json
    {
      "spec_version": "1.0",
      "event_id": "evt_01J...",
      "event_type": "user.updated",
      "environment_id": "env_...",
      "occurred_at": "2026-11-18T00:00:00Z",
      "aggregate_type": "application_user",
      "aggregate_id": "app_usr_...",
      "aggregate_version": 4,
      "data": {
        "sub": "usr_...",
        "app_user_id": "app_usr_...",
        "email": "alice@example.com",
        "email_verified": true,
        "display_name": "Alice",
        "avatar_url": null,
        "status": "active",
        "public_metadata": {},
        "version": 4
      }
    }
    ```
- **Canonical Event Types (Phase 05 core + Phase 08 org expansion ready):**
  - Identity & Membership: `user.created`, `user.updated`, `user.deleted`, `user.suspended`, `user.email_verified`, `membership.created`, `membership.updated`, `membership.revoked`
  - Sessions & Access: `session.created`, `session.revoked`, `role.assigned`, `role.revoked`
  - Organizations (emitted in Phase 08): `organization.created`, `organization.updated`, `organization.deleted`, `organization.member_added`, `organization.member_updated`, `organization.member_removed`
- **Global-to-Tenant Fan-Out Rule:**
  - When a global `users` attribute changes (e.g., `display_name`, `primary_email`, `avatar_url`, or global `deleted`), the outbox fan-out emits a scoped event **only to environments where that user has an `application_users` membership**—never leaking global users to unrelated applications.

---

## 2. SSRF-Safe Webhook Delivery Worker

Build an independently runnable Rust worker binary (`apps/auth-api/src/bin/webhook_worker.rs`) sharing the domain/repository crate so it scales independently from HTTP API pods:

- **Queue Claiming & Idempotency:**
  - Claim pending `outbox_events` batches using `SELECT ... FOR UPDATE SKIP LOCKED` for safe multi-replica worker concurrency.
  - Create per-endpoint `webhook_deliveries` records keyed by `(webhook_endpoint_id, outbox_event_id)` for idempotent dispatch.
- **Cryptographic Signatures & Replay Protection:**
  - Envelope-encrypted per-endpoint signing secret (`whsec_...`) with dual-secret rotation support (sign with both active + retiring secret during rotation window).
  - HTTP headers on every delivery:
    - `X-Auth-Event-Id: evt_...` (consumer deduplication key)
    - `X-Auth-Timestamp: <unix_epoch_secs>` (consumer rejects > 5 min skew)
    - `X-Auth-Signature: v1=<hex_hmac_sha256(timestamp + "." + raw_body)>`
- **Connection-Time SSRF Defense (Mandatory):**
  - Validate HTTPS scheme on endpoint registration (allow `http://localhost` only in `development` environments).
  - Implement a custom `reqwest` DNS resolver / socket validator that inspects **resolved IP addresses at connection time** and blocks loopback (`127.0.0.0/8`, `::1`), RFC1918 private networks (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), link-local/cloud metadata (`169.254.0.0/16`, `fe80::/10`), and carrier-grade NAT (`100.64.0.0/10`) in production—defending against DNS rebinding.
  - Disable automatic HTTP redirects (`redirect::Policy::none()`), enforce strict connect/read timeouts (e.g., 3s connect, 10s total), and cap stored response body excerpts (e.g., 4 KB).
- **Retry, Backoff, Circuit Breaking & Dead-Letter Queue (DLQ):**
  - Exponential backoff with jitter (e.g., 10s, 30s, 2m, 10m, 30m, 2h, 6h, 12h) for 5xx/429/network errors; treat 2xx as success; treat permanent 4xx (except 408/429) with shorter retry cap.
  - Transition exhausted deliveries to `dead_letter`; auto-pause consistently failing endpoints after threshold and alert developer.
  - Support manual replay of individual deliveries or time-range replay via Control Plane API.

---

## 3. Backend Sync & Reconciliation API (`/v1/backend/*`)

Webhooks alone are eventually consistent and can be missed during consumer outages or initial migrations. Provide a first-class **Pull & Reconciliation API** authenticated by `sk_test_*` / `sk_live_*`:

- **User & Membership Sync Endpoints:**
  - `GET /v1/backend/users?updated_since=<iso8601>&cursor=<opaque>&limit=100` — deterministic keyset-paginated list ordered by `(updated_at, id)` returning `sub`, `app_user_id`, `email`, `email_verified`, `display_name`, `avatar_url`, `status`, `roles`, `public_metadata`, `private_metadata`, `external_id`, `version`, `updated_at`.
  - `GET /v1/backend/users/:app_user_id` and `GET /v1/backend/users/by-sub/:sub`
  - `PATCH /v1/backend/users/:app_user_id/metadata` — set `external_id`, `public_metadata`, `private_metadata` from consuming backend.
  - `POST /v1/backend/users/:app_user_id/suspend` / `/restore`
- **Event Log Catch-Up Endpoint:**
  - `GET /v1/backend/events?after_event_id=<evt_id>&types=user.updated,session.revoked&limit=100` — lets a consuming service catch up on missed events after downtime using a durable cursor.
- **Session Inspection & Revocation Endpoints:**
  - `GET /v1/backend/sessions/:session_id` (authoritative active/revoked status for high-risk mutations)
  - `POST /v1/backend/sessions/:session_id/revoke`
  - `POST /v1/backend/users/:app_user_id/sessions/revoke-all`
- **Control Plane Webhook Management Endpoints (`/v1/control/applications/:app_id/environments/:env_kind/webhooks`):**
  - Register/update/pause/delete endpoints, rotate signing secrets, send synthetic `webhook.test` ping, inspect delivery history, and trigger manual replay.

---

## Required Tests & Verification Gates (Executed in Phase 05)

- **Transactional Atomicity Test:** Roll back a user update transaction -> verify zero `outbox_events` row is written. Commit -> verify exact event with incremented `aggregate_version`.
- **Global Fan-Out Isolation Test:** Update global user belonging to App A (`Ferrum`) but NOT App B (`Elysiar`) -> verify webhook fires for App A and **never** for App B.
- **Signature & Replay Verification Tests:** Valid HMAC verifies; tampered body, tampered timestamp, or expired timestamp fails verification; dual-secret rotation accepts both old and new signatures during grace window.
- **SSRF Defense Tests:** Mock DNS/URLs pointing to `169.254.169.254`, `127.0.0.1`, `10.0.0.1`, `::1`, and HTTP `302` redirect to internal IP -> verify worker blocks all of them in production mode.
- **Retry, DLQ & Replay Tests:** Simulate 500 -> verify exponential backoff scheduling -> exhaust retries -> verify `dead_letter` -> trigger manual replay -> verify successful delivery on consumer recovery.
- **Cursor Reconciliation Tests:** Create/update 250 users across multiple environments -> paginate via `/v1/backend/users?updated_since=...` with `sk_live_*` -> verify complete, duplicate-free, strictly environment-isolated sync results.

---

## Deliverables

- Transactional outbox producer integrated into Phase 03/04 domain services
- Standalone `webhook_worker` Rust binary with SSRF-safe HTTP client, HMAC signing, backoff/DLQ, and graceful shutdown
- Backend Sync & Reconciliation API (`/v1/backend/*`)
- Control Plane Webhook Management API (`/v1/control/.../webhooks`)
- Consumer Sync Contract Guide (`docs/service-synchronization.md`) covering push + JIT + pull reconciliation + idempotent upsert SQL patterns

---

## Acceptance Criteria

- [ ] Every identity, membership, and session lifecycle mutation atomically records an outbox event with monotonic `aggregate_version`.
- [ ] Global user updates fan out strictly to environments where that user holds an `application_users` membership.
- [ ] Webhook worker verifies resolved IPs at connection time, blocks redirects, signs payloads with HMAC-SHA256 + timestamp, retries with exponential backoff + jitter, and supports manual replay.
- [ ] Consuming services can bootstrap or reconcile their full user table via `/v1/backend/users?updated_since=...` and `/v1/backend/events?after_event_id=...` using `sk_*` keys.
- [ ] All Phase 05 isolation, SSRF, signature, DLQ, and reconciliation integration tests pass.

---

## Out of Scope

- Organization event emissions (added in Phase 08 onto this outbox)
- Visual webhook log inspector UI in Dashboard (Phase 07 — though Control Plane API is ready here)

