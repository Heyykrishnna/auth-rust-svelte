# Phase 01: Multi-Tenant Identity & Database Foundation

- **Status:** Completed
- **Prerequisites:** Phase 00 completed (ADRs finalized, transaction helpers in place)
- **Primary Targets:** `apps/auth-api/migrations/`, `apps/auth-api/src/models/`, `apps/auth-api/src/repositories/`, `apps/auth-api/src/middleware/`, `apps/auth-api/tests/`

---

## Objective

Transform the single-application PostgreSQL schema and Rust data access layer into a strictly isolated multi-tenant identity foundation where global identities (`users`) can belong to multiple independent applications and environments (`application_environments` -> `application_users`) with database-enforced tenant boundaries and zero cross-tenant leakage.

---

## 1. Database Schema & Migrations

Create versioned, reversible SQL migrations introducing the full multi-tenant foundation while preserving clean migration paths from existing tables (`001`–`006`):

### Control Plane & Tenant Tables
- `developers`: `id`, `email`, `password_hash`, `display_name`, `email_verified`, `status`, timestamps (or link to global `users` per ADR-003).
- `developer_workspaces` & `developer_workspace_members`: supports multi-developer teams (`owner`, `admin`, `developer`, `viewer`) rather than rigid single-owner apps.
- `applications`: `id`, `workspace_id`, `slug` (unique), `name`, `status` (`active`, `suspended`, `deleted`), `metadata` (`JSONB`), timestamps.
- `application_environments`: `id`, `application_id`, `kind` (`development`, `production`), `slug`, `enrollment_policy` (`consent_required`, `auto_join`, `invite_only`), `session_ttl_secs`, `access_token_ttl_secs`, `require_email_verification`, `require_mfa`, `branding` (`JSONB`), `settings` (`JSONB`), `status`, timestamps. Unique `(application_id, kind)`.
- `application_domains`: `id`, `environment_id`, `domain`, `is_verified`, `verification_token`, `allowed_origins` (`TEXT[]`), `redirect_uris` (`TEXT[]`), `post_logout_redirect_uris` (`TEXT[]`), timestamps.
- `application_credentials`: `id`, `environment_id`, `credential_type` (`publishable`, `secret`), `key_prefix` (`pk_test_`, `pk_live_`, `sk_test_`, `sk_live_`), `public_key` (nullable, indexed for publishable keys), `secret_hash` (nullable, indexed SHA-256/HMAC hash for secret keys), `name`, `scopes` (`TEXT[]`), `last_used_at`, `expires_at`, `revoked_at`, `created_by`, timestamps.
- `application_oauth_providers`: `id`, `environment_id`, `provider` (`TEXT`), `enabled`, `client_id`, `client_secret_encrypted`, `scopes` (`TEXT[]`), `use_platform_credentials` (`BOOLEAN`), timestamps.

### Global Identity Plane Tables
- `users`: `id` (immutable global `sub`), `primary_email_id`, `password_hash`, `display_name`, `avatar_url`, `status` (`active`, `suspended`, `pending_deletion`, `deleted`), `version` (`BIGINT NOT NULL DEFAULT 1` — monotonic version for service sync), timestamps.
- `user_emails`: `id`, `user_id`, `email` (citext / lowercased normalized), `verified_at`, `is_primary`, timestamps. Unique normalized `email` across verified/active accounts to prevent duplicate ownership and unverified account-linking takeovers.
- `oauth_accounts`: `id`, `user_id`, `provider` (`TEXT`, replacing Postgres ENUM), `provider_user_id`, `email`, `email_verified`, `access_token_encrypted`, `refresh_token_encrypted`, `expires_at`, timestamps. Unique `(provider, provider_user_id)`.
- `sso_sessions`: `id`, `user_id`, `session_token_hash` (unique), `authenticated_at`, `amr` (`TEXT[]`, e.g., `['pwd']`, `['pwd','otp']`), `acr`, `user_agent`, `ip_address`, `expires_at`, `last_seen_at`, `revoked_at`, timestamps.

### Tenant-Scoped Membership, Session & Authorization Tables
- `application_users`: `id` (stable `app_user_id`), `environment_id`, `user_id`, `status` (`active`, `suspended`, `revoked`), `external_id` (optional mapping to consuming service ID), `public_metadata` (`JSONB`), `private_metadata` (`JSONB`), `version` (`BIGINT NOT NULL DEFAULT 1`), `joined_at`, `last_active_at`, timestamps.
  - **Unique constraints:** `(environment_id, user_id)` and unique `(environment_id, id)` to support composite FKs.
- `app_sessions`: `id`, `environment_id`, `application_user_id`, `user_id`, `sso_session_id` (nullable FK to `sso_sessions` for global logout linkage), `session_hash`, `user_agent`, `ip_address`, `active_organization_id` (nullable), `expires_at`, `last_used_at`, `revoked_at`, timestamps.
  - **Composite FK:** `(environment_id, application_user_id)` references `application_users(environment_id, id)` `ON DELETE CASCADE`.
- `refresh_tokens`: `id`, `environment_id`, `app_session_id`, `user_id`, `token_hash` (unique), `family_id`, `expires_at`, `revoked_at`, timestamps.
- `organizations`: `id`, `environment_id`, `slug`, `name`, `logo_url`, `status`, `metadata` (`JSONB`), `version` (`BIGINT NOT NULL DEFAULT 1`), timestamps. Unique `(environment_id, slug)` and `(environment_id, id)`.
- `organization_members`: `id`, `environment_id`, `organization_id`, `application_user_id`, `status`, `version` (`BIGINT NOT NULL DEFAULT 1`), timestamps.
  - **Composite FKs:** `(environment_id, organization_id)` -> `organizations(environment_id, id)` and `(environment_id, application_user_id)` -> `application_users(environment_id, id)`.
- `organization_invitations`: `id`, `environment_id`, `organization_id`, `email`, `role_id`, `token_hash` (unique), `invited_by_app_user_id`, `status` (`pending`, `accepted`, `revoked`, `expired`), `expires_at`, `accepted_at`, timestamps.
- `roles`, `permissions`, `role_permissions`, `application_user_roles`, `organization_member_roles`: Scoped by `environment_id` (with system default role seeds per new environment).

### Platform Infrastructure & Sync Tables
- `signing_keys`: `id`, `kid` (unique), `algorithm` (`RS256` / `ES256`), `public_jwk` (`JSONB`), `private_key_encrypted` (`BYTEA`), `kek_version`, `status` (`active`, `next`, `retiring`, `revoked`), `activated_at`, `retires_at`, `revoked_at`, timestamps.
- `authentication_transactions`: `id`, `environment_id`, `flow_type` (`oidc_authorize`, `social_oauth`, `email_otp`, `password_reset`), `state_hash`, `nonce`, `code_challenge`, `code_challenge_method`, `redirect_uri`, `scope`, `authorization_code_hash`, `user_id`, `status`, `expires_at`, `consumed_at`, timestamps.
- `outbox_events`: `id`, `environment_id`, `event_type`, `aggregate_type`, `aggregate_id`, `aggregate_version` (`BIGINT`), `payload` (`JSONB`), `status` (`pending`, `processing`, `published`, `dead_letter`), `available_at`, `attempts`, timestamps.
- `webhook_endpoints`: `id`, `environment_id`, `url`, `description`, `subscribed_events` (`TEXT[]`), `signing_secret_encrypted`, `status` (`active`, `paused`, `disabled`), timestamps.
- `webhook_deliveries`: `id`, `environment_id`, `webhook_endpoint_id`, `outbox_event_id`, `attempt_number`, `request_headers` (`JSONB`), `response_status`, `response_body_excerpt`, `latency_ms`, `status`, `next_retry_at`, `delivered_at`, timestamps.
- `audit_logs`: `id`, `environment_id` (nullable for global/control-plane events), `actor_type` (`user`, `developer`, `secret_key`, `system`), `actor_id`, `event` (`TEXT`, replacing ENUM), `resource_type`, `resource_id`, `ip_address`, `user_agent`, `metadata` (`JSONB`), `created_at`.

---

## 2. Database Security, RLS & Indexing

- **Composite Foreign Keys:** Enforce cross-table tenant consistency at the schema level so an `app_session`, `organization_member`, or `role_assignment` can never link records from two different environments.
- **PostgreSQL Row-Level Security (RLS) with Connection Pooling Safety:**
  - Enable RLS on tenant-scoped tables using `current_setting('app.current_environment_id', true)::uuid`.
  - Provide a dedicated Rust repository helper `with_tenant_tx(pool, environment_id, |tx| ...)` that executes `SET LOCAL app.current_environment_id = $1` inside an explicit transaction—**never** session-level `SET`, which leaks across pooled SQLx connections.
  - Separate global IdP queries (`users`, `user_emails`, `sso_sessions`) into explicitly named global repository modules rather than bypassing tenant boundaries ad hoc.
- **High-Frequency Indexes:** Index `(environment_id, user_id)`, `(environment_id, updated_at, id)` (for cursor sync in Phase 05), `user_emails(lower(email))`, `application_credentials(public_key)`, `application_credentials(secret_hash)`, `outbox_events(status, available_at)`, and partial active indexes (`WHERE revoked_at IS NULL`).
- **Monotonic `version` Triggers:** Automatically increment `version = OLD.version + 1` on updates to `users`, `application_users`, `organizations`, and `organization_members` so consuming services always receive strict monotonic versions for idempotent sync.

---

## 3. Rust Domain Models, Tenant Context & Repositories

- Define strong newtypes (`UserId`, `ApplicationId`, `EnvironmentId`, `AppUserId`, `OrganizationId`, `SsoSessionId`, `AppSessionId`) to prevent accidentally swapping UUID arguments across layers.
- Build `TenantContext` extractor/middleware resolving `EnvironmentId` from publishable key, secret key, or OIDC `client_id`.
- Refactor `repositories/users.rs`, `repositories/sessions.rs`, `repositories/refresh_tokens.rs`, `repositories/authorization.rs`, and `repositories/audit_logs.rs` into cleanly separated **Global Identity Repositories** and **Environment-Scoped Repositories**.
- Ensure all Redis keys are namespace-prefixed by `env:{environment_id}:...` (or `idp:global:...` for global SSO/lockouts) to eliminate cache-level tenant collisions.

---

## Required Tests & Verification Gates (Executed in Phase 01)

- **Migration Up/Down & Data Migration Tests:** Clean database boot from `001` -> latest, plus migration from an existing populated single-tenant database to a default seeded application/environment without data loss.
- **Database Constraint Tests:** Verify Postgres rejects cross-environment `app_sessions` and `organization_members` inserts via composite foreign keys.
- **RLS & Pool Leakage Tests:** Run concurrent async tasks across Environment A and Environment B sharing a small `PgPool` (`max_connections = 2`) and assert zero cross-tenant reads/writes or `SET LOCAL` bleed-through.
- **Email Normalization & Verification Uniqueness Tests:** Verify case-insensitive uniqueness and rejection of unverified email hijacking.
- **Monotonic Version Tests:** Verify `version` increments deterministically on every update to synced entities.

---

## Deliverables

- Versioned SQL migrations + rollback documentation (`docs/migrations.md`)
- Strongly-typed domain models and ID newtypes (`apps/auth-api/src/models/`)
- Global and tenant-scoped repository modules with `with_tenant_tx` helper (`apps/auth-api/src/repositories/`)
- Initial `TenantContext` type and extraction skeleton
- Passing Phase 01 integration and tenant-isolation test suite

---

## Acceptance Criteria

- [x] All tables in the expanded ER architecture exist in versioned SQL migrations with proper primary keys, composite foreign keys, check constraints, and indexes.
- [x] Two distinct environments (and two distinct applications) can both reference the same global `users` row via separate `application_users` rows while keeping their `app_sessions`, `roles`, `organizations`, and `audit_logs` completely isolated.
- [x] Attempting to attach an `app_session` or `organization_member` from Environment A to an `application_user` from Environment B fails at the database constraint level.
- [x] Concurrent pooled-connection RLS tests pass with zero cross-environment leakage.
- [x] All Redis key builders enforce explicit `idp:global:` or `env:{id}:` prefixes.
- [x] `cargo test`, `cargo clippy -- -D warnings`, and migration validation pass.

---

## Out of Scope

- Developer-facing HTTP endpoints for creating applications/keys (Phase 02)
- OIDC Authorization Code endpoints and Hosted UI (Phase 04)
- Webhook delivery worker runtime (Phase 05)

