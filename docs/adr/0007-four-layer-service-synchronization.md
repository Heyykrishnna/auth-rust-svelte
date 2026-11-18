---
status: accepted
---

# 0007: Four-Layer Service Synchronization Contract

Consuming applications (such as Ferrum and Elysiar) maintain local domain databases in sync with the Identity Platform through four complementary layers rather than relying on webhooks alone:

1. **Push Sync (Transactional Outbox + Signed Webhooks)**: Domain mutations atomically write `outbox_events` with a strictly increasing `aggregate_version`; an independent SSRF-safe `webhook_worker` delivers `v1` HMAC-SHA256 signed events with timestamps, `event_id` deduplication keys, exponential backoff + jitter, DLQ, and replay.
2. **Just-In-Time (JIT) Claim Sync**: Asymmetric Access/ID tokens and `/oauth2/userinfo` carry immutable `sub`, `app_user_id`, `email`, `email_verified`, `org_id`, `roles`, `permissions`, and `ver` so consuming backends can upsert a user immediately on first login without racing a webhook.
3. **Pull & Reconciliation Sync**: Keyset-paginated `/v1/backend/users?updated_since=...`, `/v1/backend/organizations?updated_since=...`, and `/v1/backend/events?after_event_id=...` endpoints allow fresh bootstraps and recovery after prolonged downtime.
4. **Revocation Sync**: Short-lived asymmetric JWTs (default 5m, configurable down to 60s), `session.revoked` webhooks, OIDC Back-Channel Logout, and `/v1/backend/sessions/:id` introspection for high-risk actions.

## Considered Options

- **Webhooks only**: Drops or delays first-login user creation during race conditions and has no clean recovery mechanism after prolonged consumer outages.
- **Synchronous HTTP lookups on every request**: Turns the Identity Platform into a hard runtime latency and availability bottleneck for every consuming service API call.
- **Four-layer hybrid (Outbox Webhooks + JIT Claims + Pull Reconciliation + Short-Lived JWT Revocation)**: Guarantees eventual convergence, zero first-login race conditions, low-latency stateless request verification via JWKS, and deterministic out-of-order protection.

## Consequences

- Consuming services **must** key local user projections on immutable `sub` or `app_user_id` (never mutable email) and guard upserts with `incoming.version >= local.identity_version`.
- Global `users` mutations fan out `outbox_events` strictly to environments where that user holds an `application_users` row.
- PostgreSQL triggers automatically increment `version` on every update to `users`, `application_users`, `organizations`, and `organization_members`.

