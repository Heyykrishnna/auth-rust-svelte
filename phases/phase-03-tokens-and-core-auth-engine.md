# Phase 03: Asymmetric Token Infrastructure & Multi-Tenant Auth Engine

- **Status:** Completed
- **Prerequisites:** Phase 02 completed (environments, credentials, dynamic CORS, tenant extraction)
- **Primary Targets:** `apps/auth-api/src/services/tokens.rs`, `apps/auth-api/src/services/authentication.rs`, `apps/auth-api/src/services/sessions.rs`, `apps/auth-api/src/services/authorization.rs`, `apps/auth-api/src/handlers/`, `apps/auth-api/src/routes/`

---

## Objective

Replace the single-tenant HS256 authentication engine with an **asymmetric-key (RS256/ES256 + JWKS)**, multi-tenant authentication, session, and RBAC engine so two independent applications can authenticate global identities with strictly isolated sessions, token audiences, and permissions—laying the cryptographic bedrock for Phase 04 OIDC SSO.

---

## 1. Asymmetric Token Infrastructure & Key Management (Pulled Ahead of OIDC)

- **Envelope-Encrypted Signing Keys (`signing_keys`):**
  - Generate `ES256` (ECDSA P-256, compact & fast) and/or `RS256` (broad compatibility) keypairs with unique `kid` headers.
  - Encrypt private keys at rest using AES-256-GCM envelope encryption backed by a master KEK (env/KMS-sourced, versioned via `kek_version`). Never store plaintext private keys in PostgreSQL.
- **Public JWKS Endpoint:**
  - `GET /.well-known/jwks.json` returning all `active` and `retiring` public keys with proper `Cache-Control` headers (`public, max-age=300`) and `ETag`.
  - Support zero-downtime key rotation (`next` -> `active` -> `retiring` -> `revoked`) so tokens signed with the previous key verify cleanly until they expire.
- **Strict Token Claims & Isolation:**
  - Separate **Access Tokens** (`typ: "at+jwt"`) and **ID Tokens** (`typ: "JWT"`, used in Phase 04).
  - Required access token claims: `iss` (platform issuer URL), `sub` (immutable global `user_id`), `aud` (`environment_id` / app client ID), `azp` (authorized party), `app_user_id` (environment membership ID), `sid` (`app_session_id`), `org_id` (nullable active org), `roles`, `permissions`, `ver` (identity/membership version for JIT sync), `iat`, `exp`, `jti`, `kid` (header).
  - Short-lived access tokens (default 5 minutes; configurable down to 60 seconds per environment).
  - Remove all legacy `HS256` token signing/verification from the auth engine.

---

## 2. Multi-Tenant Core Authentication Engine (`/v1/client/*`)

Refactor `services/authentication.rs` into cleanly modular, transactional, environment-aware flows:

- **Application-Aware Sign-Up & Global Identity Handling:**
  - Normalize email (`trim`, lowercase).
  - If email does not exist globally: run OTP email verification -> atomically create `users` + `user_emails` + `application_users` (for target `environment_id`) + default role + `app_session` + `refresh_token` + `outbox_events` (`user.created`, `membership.created`) + `audit_logs`.
  - **Global Email Collision Handling:** If the email already belongs to a global `users` identity (e.g., user previously registered on Ferrum and now signs up on Elysiar), **never** overwrite the existing password or disclose raw account metadata blindly; route cleanly through authentication of the existing global identity and attach the new `application_users` membership upon verification.
- **Email & Password Sign-In:**
  - Constant-time / timing-normalized Argon2id verification, Redis-backed brute-force lockout (per-IP + per-email), verification of both global `users.status == 'active'` and environment `application_users.status == 'active'`.
  - Respect environment `enrollment_policy` when a valid global user signs into a new environment for the first time.
- **Email Verification, OTP & Password Lifecycle:**
  - Registration OTP verification & rate-limited resend.
  - Adding/verifying secondary emails (`user_emails`).
  - Password reset (single-use hashed token, revokes all `app_sessions` and `sso_sessions` on completion), password change (requires current password).
  - Per-environment email branding (app name, logo, support contact from `application_environments.branding`).
- **Account Status & Lifecycle:**
  - Environment-level user suspension/restoration (`application_users.status`) vs. global account suspension/deletion (`users.status`).
- **MFA-Ready Foundation:**
  - Record `amr` (`["pwd"]`, `["oauth"]`, `["pwd","otp"]`) and `acr` on sessions and `authentication_transactions` so TOTP/WebAuthn step-up slots in cleanly without schema breaks.

---

## 3. Upstream Social Authentication (Google & GitHub)

Replace the stubbed `handle_oidc_callback` in `apps/auth-api/src/services/authentication.rs` with a production upstream OAuth2/OIDC client:

- Real Authorization Code Flow with **PKCE (`S256`)**, persisted single-use **`state`**, and **`nonce`** (for Google ID token verification) stored in `authentication_transactions` / Redis with short TTL (10 minutes).
- Support platform-default credentials or environment-specific `application_oauth_providers` credentials (decrypted via envelope encryption).
- **Safe Account Linking Rules:**
  - Match existing `(provider, provider_user_id)` in `oauth_accounts` first.
  - Allow automatic linking by email **only** if both the upstream provider asserts `email_verified == true` AND the local `user_emails` record is verified. Never auto-link on unverified emails.
  - Ensure `application_users` membership is created/verified for the active `environment_id` before issuing app tokens.

---

## 4. Environment-Scoped Session & Refresh Token Infrastructure

- **Application Sessions (`app_sessions`):**
  - Strictly scoped to `(environment_id, application_user_id)` with metadata (`user_agent`, `ip_address`, `last_used_at`, `expires_at`, `revoked_at`).
  - Multi-device support: list active sessions for current app, revoke individual session, revoke all sessions for current app.
- **Refresh Token Rotation & Reuse Detection (RFC 6819):**
  - Opaque high-entropy refresh tokens (hashed with SHA-256 in `refresh_tokens`, bound to `environment_id`, `app_session_id`, and `family_id`) rather than bloated JWT refresh tokens.
  - Atomic rotation inside a DB transaction (`SELECT ... FOR UPDATE`): presenting an already-rotated/revoked token triggers immediate revocation of the entire `family_id` and its `app_session`, plus a high-severity `REFRESH_TOKEN_REUSE_DETECTED` audit event.
  - Every refresh re-evaluates `users.status`, `application_users.status`, and current roles/permissions so suspensions and role changes take effect immediately on refresh.

---

## 5. Environment-Scoped RBAC & Authorization Middleware

- Refactor `services/authorization.rs` and `middleware/authorization.rs` to evaluate roles and permissions strictly within the active `environment_id`.
- Remove the insecure default-permission fallback in `apps/auth-api/src/middleware/auth.rs` (missing claims must fail closed, never grant default permissions).
- Versioned Client API routes:
  - `POST /v1/client/auth/sign-up`, `/sign-up/verify`, `/sign-up/resend-otp`
  - `POST /v1/client/auth/sign-in`, `/sign-out`, `/refresh`
  - `POST /v1/client/auth/verify-email`, `/password/forgot`, `/password/reset`, `/password/change`
  - `GET /v1/client/auth/oauth/:provider/start`, `POST /v1/client/auth/oauth/:provider/callback`
  - `GET/PATCH /v1/client/users/me`
  - `GET/DELETE /v1/client/sessions`, `DELETE /v1/client/sessions/:id`

---

## Required Tests & Verification Gates (Executed in Phase 03)

- **Asymmetric JWT & JWKS Tests:** Verify tokens validate with public key from `/.well-known/jwks.json`; verify key rotation validates both active and retiring `kid`s; verify tokens with wrong `aud` or `iss` are rejected.
- **Cross-App Token & Session Isolation Tests:** Access token and refresh token issued for App A (`Ferrum`) are **strictly rejected** on App B (`Elysiar`) endpoints. Revoking a user's session or suspending their membership in App A does not invalidate their active session in App B.
- **Refresh Reuse & Concurrency Tests:** Replaying a consumed refresh token revokes the entire family; concurrent refresh requests do not issue two valid live descendants.
- **Upstream OAuth Security Tests:** Invalid/expired/replayed `state`, missing PKCE, invalid `nonce`, and unverified-email auto-linking attempts are all rejected.
- **Global Password Reset Propagation Test:** Resetting password on global `users` revokes active `app_sessions` across all environments.

---

## Deliverables

- Asymmetric signing key service + envelope encryption + `GET /.well-known/jwks.json`
- Multi-tenant authentication engine & versioned `/v1/client/*` API
- Production Google & GitHub OAuth2/OIDC client integration
- Environment-scoped session, refresh rotation, and RBAC services
- Passing Phase 03 unit, integration, concurrency, and cross-app isolation test suites

---

## Acceptance Criteria

- [x] `GET /.well-known/jwks.json` serves valid RS256/ES256 public keys and supports zero-downtime key rotation.
- [x] Zero HS256 secrets remain in access/ID token issuance or verification.
- [x] Two independent applications (`Ferrum` and `Elysiar`) can register and authenticate the same global user with completely isolated `app_sessions`, `aud`-scoped JWTs, and environment-specific roles.
- [x] Refresh token reuse revokes the entire token family atomically and records a security audit event.
- [x] Google and GitHub OAuth callbacks execute real PKCE + `state` + `nonce` validation and enforce verified-email-only account linking.
- [x] All Phase 03 security and tenant-isolation tests pass.

---

## Out of Scope

- Downstream OIDC Provider endpoints (`/oauth2/authorize`, `/oauth2/token`) and central `sso_sessions` (Phase 04)
- Webhook delivery workers (Phase 05 — though outbox rows can already be written transactionally if schema is present)

