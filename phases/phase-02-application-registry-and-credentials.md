# Phase 02: Application Registry, Environments, Credentials & Dashboard Shell

- **Status:** Completed
- **Prerequisites:** Phase 01 completed (multi-tenant schema, `application_environments`, `application_credentials`, `application_domains`)
- **Primary Targets:** `apps/auth-api/src/routes/`, `apps/auth-api/src/handlers/`, `apps/auth-api/src/middleware/`, `apps/web/`

---

## Objective

Enable developers to register applications with isolated **Development** and **Production** environments, mint/rotate publishable and secret credentials, configure dynamic allowed origins and exact redirect URIs, and manage them through an initial Developer Control Plane shell—without touching internal platform configuration.

---

## 1. Developer Control Plane API (`/v1/control/*`)

- **Developer Authentication & Workspace Management:**
  - Developer sign-up / sign-in / session management (per ADR-003).
  - Workspace creation and developer membership checks (`owner`, `admin`, `developer`, `viewer`) enforced via workspace authorization middleware.
- **Application Registry Endpoints:**
  - `POST /v1/control/applications` — creates application + default `development` and `production` environments + initial publishable key + default roles (`user`, `admin`) in one atomic transaction.
  - `GET /v1/control/applications` / `GET /v1/control/applications/:app_id`
  - `PATCH /v1/control/applications/:app_id` — name, slug, metadata, activation/suspension.
  - `DELETE /v1/control/applications/:app_id` — soft-delete / suspension workflow with credential invalidation.
- **Environment Configuration Endpoints:**
  - `GET/PATCH /v1/control/applications/:app_id/environments/:env_kind` — configure `enrollment_policy` (`consent_required`, `auto_join`, `invite_only`), session TTLs, access token TTLs, password policy, email verification requirement, branding (`name`, `logo_url`, `primary_color`, `support_email`).

---

## 2. Credential Management (`pk_*` and `sk_*`)

### Publishable Keys (`pk_test_...`, `pk_live_...`)
- Publicly safe identifiers used by browser SDKs and OIDC clients to identify an `application_environment`.
- Never grant privileged access; validated alongside origin/redirect URI policies.
- Support status (`active`, `revoked`) and zero-downtime replacement.

### Secret Keys (`sk_test_...`, `sk_live_...`)
- High-entropy CSPRNG generation (minimum 256 bits of entropy) with clear environment prefix (`sk_test_`, `sk_live_`) and non-sensitive display hint (first/last 4 chars).
- **Returned in plaintext exactly once on creation**; stored exclusively as keyed cryptographic hash (`HMAC-SHA256` with server pepper or `SHA-256` of high-entropy token) with constant-time comparison.
- Support explicit permission scopes (e.g., `users:read`, `users:write`, `sessions:revoke`, `orgs:manage`, `sync:read`), rotation with grace window, immediate revocation, `last_used_at` tracking (debounced via Redis to avoid write amplification), and full audit logging.

### Credential Authentication & Tenant Resolution Middleware
- `PublishableKeyTenant` extractor (for `/v1/client/*`): resolves `EnvironmentId` (Redis-cached with invalidation on key revoke/app suspend).
- `SecretKeyAuth` extractor (for `/v1/backend/*`): verifies `Authorization: Bearer sk_...`, resolves `EnvironmentId`, enforces required scopes, rejects suspended apps/revoked keys.

---

## 3. Domain, Origin & Redirect URI Management

- `POST/GET/DELETE /v1/control/applications/:app_id/environments/:env_kind/domains`
- **Environment-specific policies:**
  - `development` permits `http://localhost:*` and `http://127.0.0.1:*` (explicit ports only, no wildcard schemes).
  - `production` requires strict `https://` origins and exact-match redirect URIs (no wildcards, no open redirects, no fragment components per RFC 6749/9700).
- **Dynamic CORS Middleware:**
  - Replace the static boot-time `CorsLayer` in `apps/auth-api/src/routes/mod.rs` with a dynamic CORS layer that validates `Origin` against the resolved environment's Redis-cached `allowed_origins` (plus the platform's own Dashboard/Hosted Portal origins for Control Plane routes).
- **Domain Ownership Verification (Production):** Generate DNS TXT verification records for custom production domains.

---

## 4. Developer Control Plane Shell (`apps/web`)

Establish the foundational Control Plane dashboard structure **now** (avoiding duplicate foundation work in Phase 07):

- Developer login/registration routes under the Control Plane layout.
- Persistent shell navigation with **Workspace -> Application -> Environment (`Development` / `Production`) switcher** clearly indicating active environment.
- Application Creation Wizard (name, slug, first redirect URI, immediate copyable `pk_test_*` and one-time `sk_test_*` reveal modal).
- Application Overview & Settings view (status, enrollment policy, TTLs, branding).
- API Keys management view (publishable keys, secret key creation with one-time display, rotation, revocation, last-used indicator).
- Domains & Redirect URIs management view (with live validation feedback for dev vs prod rules).

---

## Security Requirements

- Every Control Plane route enforces workspace membership and role authorization; developer A cannot read, mutate, or rotate keys for developer B's workspace/application.
- Secret keys are never logged in HTTP tracing, error messages, or audit log metadata.
- Rate-limit credential creation, rotation, and failed secret-key authentication attempts.
- Revoking a secret key or suspending an application invalidates Redis credential caches immediately.

---

## Required Tests & Verification Gates (Executed in Phase 02)

- **Tenant & Developer Isolation Tests:** Developer A attempting to access/modify/rotate credentials or domains belonging to Developer B receives `403`/`404`.
- **Credential Lifecycle Tests:** Secret key plaintext returned only on creation; hashed in DB; revoked key immediately fails `SecretKeyAuth`; scope enforcement rejects under-scoped `sk_*` keys; `sk_test_*` cannot access `production` environment resources.
- **Dynamic CORS & Redirect Validation Tests:** Allowed origin receives CORS headers; unlisted origin is rejected; wildcard/HTTP production redirect URIs and open-redirect attempts are rejected.
- **Frontend Tests:** Vitest tests for app/env switcher state, one-time secret reveal modal, and domain validation forms.

---

## Deliverables

- Developer Control Plane API (`/v1/control/*`) for workspaces, apps, environments, credentials, and domains
- `PublishableKeyTenant`, `SecretKeyAuth`, and Dynamic CORS middleware
- Initial Developer Control Plane Dashboard shell in `apps/web`
- Audit events for all application, domain, and credential lifecycle mutations

---

## Acceptance Criteria

- [x] A developer can sign into the Dashboard, create an application, switch between `Development` and `Production` environments, copy `pk_test_*` / `sk_test_*`, and register allowed origins and redirect URIs without touching backend config files or restarting `auth-api`.
- [x] Dynamic CORS immediately reflects newly added or removed `allowed_origins` without server restart.
- [x] Secret keys are never retrievable after initial creation and are rejected immediately upon revocation.
- [x] All Control Plane, credential isolation, dynamic CORS, and frontend shell tests pass.

---

## Out of Scope

- Full end-user directory, webhook log viewer, and analytics charts in Dashboard (Phase 07)
- OIDC Discovery/Authorize/Token endpoints (Phase 04)

