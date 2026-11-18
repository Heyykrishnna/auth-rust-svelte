# Consuming Service Synchronization Contract

This guide defines how any consuming application (for example, **Ferrum**, **Elysiar**, or a new microservice) integrates with the Identity Infrastructure Platform so its local database stays accurately, idempotently, and securely synchronized with user, membership, role, and session state.

---

## 1. Golden Rules for Consuming Services

1. **Key on immutable identifiers, never on email:** Store `sub` (global user UUID) and `app_user_id` (environment membership UUID) as your authoritative foreign keys. Emails can change; `sub` and `app_user_id` never change.
2. **Guard every write with monotonic `version`:** Only apply an incoming update when `incoming.version >= local.identity_version`. This guarantees out-of-order webhook deliveries or overlapping pull-sync jobs can never overwrite newer state with stale state.
3. **Deduplicate webhook deliveries by `event_id`:** Persist processed `X-Auth-Event-Id` UUIDs inside the same local database transaction as your projection upsert.
4. **Verify signatures over raw bytes before parsing JSON:** Never re-serialize parsed JSON to verify HMAC signatures; verify `X-Auth-Signature` against the exact raw HTTP request body bytes and enforce a `±300s` timestamp skew window.

---

## 2. The Four-Layer Synchronization Model

| Layer | Mechanism | Primary Role |
| --- | --- | --- |
| **Layer 1 — Push Sync** | Transactional Outbox → Signed HTTP Webhooks (`webhook-worker`) | Real-time propagation of `user.*`, `membership.*`, `session.revoked`, `role.*`, and `organization.*` events |
| **Layer 2 — JIT Claim Sync** | Asymmetric ES256 JWT (`typ: "at+jwt"`) verified via `/.well-known/jwks.json` | Zero-race first-login upsert using `sub`, `app_user_id`, `roles`, `permissions`, and `ver` on the first authenticated request |
| **Layer 3 — Pull & Reconciliation** | `GET /v1/backend/users` & `GET /v1/backend/events` (`Authorization: Bearer sk_*`) | Initial database bootstrap, recovery after extended consumer downtime, and scheduled drift reconciliation |
| **Layer 4 — Revocation Sync** | Short-lived access tokens (60–300s) + `session.revoked` webhooks + `GET /v1/backend/sessions/:id` | Fast access cutoff and authoritative checks for high-risk mutations |

---

## 3. Canonical Webhook Envelope & Headers

Every outbound webhook request from `webhook-worker` carries:

| Header | Example | Purpose |
| --- | --- | --- |
| `Content-Type` | `application/json` | Canonical UTF-8 JSON body |
| `X-Auth-Event-Id` | `6f4e4df0-2f83-4d39-b532-27d70b9f6e2f` | Idempotency / deduplication key |
| `X-Auth-Timestamp` | `1763420000` | Unix epoch seconds when signed |
| `X-Auth-Signature` | `v1=<hex_hmac_sha256>` (comma-separated during rotation) | `HMAC-SHA256(secret, "{timestamp}.{raw_body}")` |

### Canonical Envelope (`spec_version: "1.0"`)

```json
{
  "spec_version": "1.0",
  "event_id": "6f4e4df0-2f83-4d39-b532-27d70b9f6e2f",
  "event_type": "user.updated",
  "environment_id": "2c8c2fd4-5781-4090-a7d0-9431d198a3f5",
  "occurred_at": "2026-11-18T10:00:00Z",
  "aggregate_type": "application_user",
  "aggregate_id": "9b9d6c2d-7e94-4a57-b310-632b8f0a4b7d",
  "aggregate_version": 4,
  "data": {
    "sub": "1f3a6d5c-5d95-4c0c-b62c-1c7e7c7ef9c1",
    "app_user_id": "9b9d6c2d-7e94-4a57-b310-632b8f0a4b7d",
    "email": "alice@example.com",
    "display_name": "Alice",
    "avatar_url": null,
    "version": 4
  }
}
```

### Dual-Secret Rotation Window

When rotating a webhook endpoint secret via `POST /v1/control/applications/:app_id/environments/:env_kind/webhooks/:endpoint_id/rotate-secret`, the worker signs outgoing requests with **both** the new active secret and the previous secret until `previous_secret_expires_at` elapses:

```text
X-Auth-Signature: v1=<sig_new>,v1=<sig_previous>
```

Consumers MUST split `X-Auth-Signature` on `,` and accept the request if **any** `v1=` signature matches any configured secret in constant time.

---

## 4. Signature Verification Reference

### TypeScript (Node.js / Edge)

```ts
import crypto from "node:crypto";

export function verifyAuthWebhookSignature(params: {
  rawBody: Buffer;
  timestampHeader: string | undefined;
  signatureHeader: string | undefined;
  secrets: string[];
  toleranceSecs?: number;
}): void {
  const tolerance = params.toleranceSecs ?? 300;
  const timestamp = Number(params.timestampHeader);
  if (!Number.isFinite(timestamp)) {
    throw new Error("Missing or invalid X-Auth-Timestamp");
  }
  const nowSecs = Math.floor(Date.now() / 1000);
  if (Math.abs(nowSecs - timestamp) > tolerance) {
    throw new Error("Webhook timestamp outside replay tolerance window");
  }

  const presented = (params.signatureHeader ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.startsWith("v1="));

  if (presented.length === 0) {
    throw new Error("Missing v1 signature");
  }

  const signedPayload = Buffer.concat([
    Buffer.from(`${timestamp}.`, "utf8"),
    params.rawBody,
  ]);

  for (const secret of params.secrets) {
    const expected = Buffer.from(
      `v1=${crypto.createHmac("sha256", secret).update(signedPayload).digest("hex")}`,
      "utf8"
    );
    for (const candidate of presented) {
      const actual = Buffer.from(candidate, "utf8");
      if (actual.length === expected.length && crypto.timingSafeEqual(actual, expected)) {
        return;
      }
    }
  }

  throw new Error("Webhook signature verification failed");
}
```

---

## 5. Idempotent Local Projection SQL Pattern (PostgreSQL)

Consuming services should maintain two local tables:

```sql
CREATE TABLE local_users (
    app_user_id       UUID PRIMARY KEY,
    sub               UUID NOT NULL UNIQUE,
    email             TEXT NOT NULL,
    email_verified    BOOLEAN NOT NULL DEFAULT FALSE,
    display_name      TEXT NOT NULL,
    avatar_url        TEXT,
    status            TEXT NOT NULL,
    roles             TEXT[] NOT NULL DEFAULT '{}',
    public_metadata   JSONB NOT NULL DEFAULT '{}'::jsonb,
    identity_version  BIGINT NOT NULL DEFAULT 0,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE processed_auth_events (
    event_id      UUID PRIMARY KEY,
    event_type    TEXT NOT NULL,
    processed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Apply webhook events atomically inside a single database transaction:

```sql
BEGIN;

INSERT INTO processed_auth_events (event_id, event_type)
VALUES ($1, $2)
ON CONFLICT (event_id) DO NOTHING;

-- If 0 rows were inserted above, commit and return HTTP 200 immediately (duplicate delivery).

INSERT INTO local_users (
    app_user_id, sub, email, email_verified, display_name, avatar_url,
    status, roles, public_metadata, identity_version, updated_at
)
VALUES ($3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
ON CONFLICT (app_user_id) DO UPDATE
SET
    email            = EXCLUDED.email,
    email_verified   = EXCLUDED.email_verified,
    display_name     = EXCLUDED.display_name,
    avatar_url       = EXCLUDED.avatar_url,
    status           = EXCLUDED.status,
    roles            = EXCLUDED.roles,
    public_metadata  = EXCLUDED.public_metadata,
    identity_version = EXCLUDED.identity_version,
    updated_at       = NOW()
WHERE EXCLUDED.identity_version >= local_users.identity_version;

COMMIT;
```

---

## 6. Bootstrap & Downtime Catch-Up Reconciliation

### Initial Full Backfill or Scheduled Drift Reconciliation

Authenticate with your environment secret key (`Authorization: Bearer sk_test_...` or `sk_live_...`, scope `sync:read` or `users:read`):

```http
GET /v1/backend/users?updated_since=2026-11-01T00:00:00Z&limit=100
Authorization: Bearer sk_live_...
```

Follow `next_cursor` until `has_more` is `false`. Records are ordered deterministically by `(updated_at ASC, app_user_id ASC)` so concurrent updates never skip rows. Feed each record through the exact same version-guarded `INSERT ... ON CONFLICT ... WHERE EXCLUDED.identity_version >= local_users.identity_version` upsert.

### Event Log Catch-Up After Outage

If your webhook consumer was offline beyond retry exhaustion (or to verify no events were missed), store the last processed `event_id` and catch up via:

```http
GET /v1/backend/events?after_event_id=<last_event_id>&types=user.updated,user.suspended,session.revoked&limit=100
Authorization: Bearer sk_live_...
```

### Authoritative Session Verification for High-Risk Actions

For sensitive operations (billing changes, API key creation, ownership transfer), verify session liveness directly against the Backend API (`sessions:read` scope):

```http
GET /v1/backend/sessions/<sid_from_jwt>
Authorization: Bearer sk_live_...
```

And immediately force-revoke sessions when your service detects abuse (`sessions:revoke` scope):

```http
POST /v1/backend/sessions/<sid>/revoke
POST /v1/backend/users/<app_user_id>/sessions/revoke-all
POST /v1/backend/users/<app_user_id>/suspend
```

---

## 7. Security & Operational Guarantees

- **Strict Environment Isolation:** Secret keys (`sk_test_*` / `sk_live_*`) are cryptographically bound to a single `environment_id`. Cross-environment lookups and event catch-up queries are impossible.
- **Global-to-Tenant Fan-Out:** Updates to global user profiles (`users`) only emit outbox events to environments where that user actually holds an `application_users` membership.
- **SSRF & DNS Rebinding Protection:** `webhook-worker` validates URLs on registration and re-validates **every DNS-resolved IP address at connection time**, blocking loopback (`127.0.0.0/8`, `::1`), RFC1918 private networks (`10/8`, `172.16/12`, `192.168/16`), link-local/cloud metadata (`169.254/16`, `fe80::/10`), CGNAT (`100.64/10`), and all HTTP `3xx` redirects.
- **Backoff, DLQ & Replay:** Transient errors retry with exponential backoff + jitter (`10s → 30s → 2m → 10m → 30m → 2h → 6h → 12h`); permanent `4xx` errors use a shorter 3-attempt cap; exhausted deliveries transition to `dead_letter`; endpoints auto-pause after 10 consecutive failures and support single-delivery or time-range manual replay via the Control Plane API.

