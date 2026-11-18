# Phase 04: OIDC Provider, Centralized SSO & Hosted Auth Portal

- **Status:** Completed
- **Prerequisites:** Phase 03 completed (RS256/ES256 + JWKS, multi-tenant auth engine, `app_sessions`, `authentication_transactions`)
- **Primary Targets:** `apps/auth-api/src/routes/oidc.rs`, `apps/auth-api/src/handlers/oidc_provider.rs`, `apps/auth-api/src/services/sso.rs`, `apps/web/` (Hosted Auth Portal)

---

## Objective

Turn the platform into a standards-compliant **OpenID Connect (OIDC) Identity Provider** with a **Hosted Auth & Consent Portal** and central **`sso_sessions`** so a user who signs into **Ferrum (App A)** can open **Elysiar (App B)** and receive Elysiar-specific tokens automatically without re-entering their password—while enforcing strict audience isolation, policy checks, and local/global logout.

---

## 1. Standards-Based OIDC Provider Endpoints (`apps/auth-api`)

Implement RFC 6749, RFC 7636 (PKCE), RFC 9700 (OAuth 2.0 Security Best Current Practice), and OpenID Connect Core 1.0:

- **Discovery Document (`GET /.well-known/openid-configuration`):**
  - Advertise `issuer`, `authorization_endpoint`, `token_endpoint`, `userinfo_endpoint`, `jwks_uri`, `end_session_endpoint`, `revocation_endpoint`, `response_types_supported: ["code"]`, `grant_types_supported: ["authorization_code", "refresh_token"]`, `code_challenge_methods_supported: ["S256"]`, `id_token_signing_alg_values_supported`, `scopes_supported: ["openid", "profile", "email", "offline_access"]`, and `claims_supported`.
- **Authorization Endpoint (`GET /oauth2/authorize`):**
  - Strictly validate `client_id` (maps to `application_environment`), exact-match `redirect_uri` against registered `application_domains.redirect_uris` (never redirect on invalid `client_id` or `redirect_uri`—render an IdP error page instead to prevent open redirects), `response_type=code`, `scope` (must include `openid`), `state`, `nonce`, and **mandatory PKCE (`code_challenge`, `code_challenge_method=S256`)** for public clients.
  - Create a short-lived `authentication_transactions` record binding all parameters.
- **Token Endpoint (`POST /oauth2/token`):**
  - Support `grant_type=authorization_code` (validating PKCE `code_verifier`, single-use `authorization_code`, exact `redirect_uri`, `client_id`, and client secret for confidential server clients) and `grant_type=refresh_token`.
  - **Code Replay Protection:** If an already-consumed authorization code is presented again, immediately revoke all tokens and the `app_session` issued from that authorization code.
  - Issue:
    - **`access_token`**: short-lived asymmetric JWT (`aud = environment_id`, `sub`, `app_user_id`, `sid`, `roles`, `permissions`, `ver`).
    - **`id_token`**: asymmetric JWT (`iss`, `sub`, `aud = client_id`, `nonce`, `auth_time`, `amr`, `acr`, `email`, `email_verified`, `name`, `picture`, `app_user_id`, `ver`).
    - **`refresh_token`**: opaque rotated token bound to the newly created `app_session` (when `offline_access` scope granted).
- **UserInfo Endpoint (`GET /oauth2/userinfo`):**
  - Validates Bearer `access_token` (requiring `openid` scope), respects granted scopes (`email`, `profile`), returns authoritative `sub`, `app_user_id`, `email`, `email_verified`, `name`, `picture`, `ver`, and environment-scoped metadata.
- **Token Revocation (`POST /oauth2/revoke`):** RFC 7009 revocation for refresh tokens and linked `app_sessions`.
- **RP-Initiated Logout (`GET/POST /oauth2/logout`):**
  - Validates `id_token_hint` and exact-match `post_logout_redirect_uri`.
  - Supports **local logout** (revokes only requesting app's `app_session`) and **global logout** (revokes IdP `sso_session` + all linked `app_sessions`, emitting `session.revoked` outbox events and optional OIDC Back-Channel Logout tokens).

---

## 2. Central `SSO_SESSIONS` & Policy Evaluation Engine

Implement the exact decision logic represented in the centralized SSO flow (`Ferrum` -> `Central IdP` -> `Elysiar`):

- **IdP Cookie Semantics:**
  - When a user authenticates on the Hosted Auth Portal, create an `sso_sessions` row and set a first-party, `HttpOnly`, `Secure` (in HTTPS), `SameSite=Lax` cookie (`__Host-idp_sso_session` in production) strictly on the Central IdP origin.
- **Deterministic SSO Policy Engine (Evaluated at `/oauth2/authorize`):**
  1. Verify `sso_session` exists, is not expired, and is not revoked.
  2. Check global `users.status == 'active'`.
  3. Evaluate OIDC parameters:
     - `prompt=login` -> force interactive re-authentication.
     - `max_age=<secs>` -> force re-authentication if `now - sso_session.authenticated_at > max_age`.
     - `prompt=none` -> if no valid SSO session or user interaction/consent is required, redirect back with `error=login_required` or `error=consent_required` (never render UI).
  4. Evaluate environment requirements:
     - If `application_users` exists and `status != 'active'` -> deny with `access_denied`.
     - If environment requires verified email and user's primary email is unverified -> route to Hosted verification step.
     - If environment requires step-up `acr`/MFA not satisfied by `sso_session.amr` -> route to step-up challenge.
  5. Evaluate `enrollment_policy` when `application_users` does **not** yet exist for the target environment (e.g., Ferrum user visiting Elysiar for the first time):
     - `auto_join` -> atomically create `application_users` + default role + `membership.created` outbox event -> issue authorization code immediately (zero password prompt).
     - `consent_required` -> show Hosted Consent UI ("Continue to **Elysiar** as `user@example.com`?") -> on approval, create `application_users` and issue code.
     - `invite_only` -> deny cleanly with an invitation-required screen/error.

---

## 3. Hosted Auth & Consent Portal UI (`apps/web`)

Build the dedicated end-user Hosted Portal served on the Central IdP origin (branded dynamically per `application_environment.branding`):

- **Hosted Sign-In Page:** Displays target app logo/name ("Sign in to **Ferrum**"), email/password form, enabled social buttons (Google/GitHub), forgot-password link, and switch-account option.
- **Hosted Sign-Up & OTP Verification Flow:** Registration + 6-digit OTP verification + smooth handling when email already exists globally.
- **Hosted Consent / App-Join Screen:** Shown on first SSO hop to a new app under `consent_required` policy, listing shared profile claims (`email`, `profile`) and active global identity.
- **Hosted Account Switcher / Logout Confirmation:** Allows users to confirm global vs. app-only sign-out and view/revoke their active IdP `sso_sessions` and connected applications.
- **Clean Separation from Developer Dashboard:** Uses dedicated routes/host routing (e.g., `/oauth/*`, `/portal/*` or `accounts.<domain>`) with strict CSP headers (`frame-ancestors 'none'` to prevent clickjacking).

---

## Security Requirements

- Mandatory PKCE (`S256`) validation; reject `plain` PKCE method.
- Authorization codes expire in **<= 60 seconds**, are cryptographically random (256 bits), stored hashed, and strictly single-use.
- Strict exact-string `redirect_uri` comparison (scheme, host, port, path, query); zero partial/prefix/wildcard matching.
- Clickjacking defense (`Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY`) on all Hosted Auth Portal pages.
- CSRF protection on all Hosted Portal form submissions bound to the `authentication_transactions` ID.

---

## Required Tests & Verification Gates (Executed in Phase 04)

- **End-to-End Dual-App SSO Test (`Ferrum` + `Elysiar`):**
  1. Authenticate user on Ferrum via `/oauth2/authorize` + `/oauth2/token` -> verify Ferrum `id_token`/`access_token` have `aud == ferrum_env_id` and `sso_session` cookie is set.
  2. Initiate `/oauth2/authorize` for Elysiar carrying the IdP `sso_session` cookie -> verify password prompt is skipped (under `auto_join` or after consent under `consent_required`) -> exchange code -> verify Elysiar tokens have `aud == elysiar_env_id` and distinct `app_session_id`.
  3. Verify Ferrum's access token is rejected on Elysiar and vice versa.
- **Policy Engine Tests:** Test `prompt=login`, `prompt=none`, `max_age=0`, `invite_only` denial, and user suspended in Elysiar while still active in Ferrum.
- **OIDC Attack Regression Tests:** Authorization code replay (verifies issued tokens/session are revoked), PKCE wrong verifier, `redirect_uri` mismatch, open-redirect probe, expired code, and `nonce` preservation in `id_token`.
- **Local vs. Global Logout Tests:** Local logout on Ferrum keeps Elysiar and `sso_session` alive; global logout revokes `sso_session` and both Ferrum and Elysiar `app_sessions`.

---

## Deliverables

- OIDC Provider backend (`/.well-known/openid-configuration`, `/oauth2/authorize`, `/oauth2/token`, `/oauth2/userinfo`, `/oauth2/revoke`, `/oauth2/logout`)
- Central `sso_sessions` service & policy engine
- Hosted Auth & Consent Portal UI in `apps/web`
- OIDC integration specification (`docs/oidc-provider.md`)

---

## Acceptance Criteria

- [x] Standard off-the-shelf OIDC clients can discover configuration via `/.well-known/openid-configuration` and verify tokens via `/.well-known/jwks.json`.
- [x] The full Image 2 flow (`Ferrum` -> Central IdP -> `Elysiar`) works end-to-end with zero second password prompt when the `sso_session` is valid and policy permits it.
- [x] First-time app joins enforce `consent_required`, `auto_join`, and `invite_only` accurately.
- [x] Authorization code replay, PKCE mismatch, and invalid `redirect_uri` attacks are blocked and audited.
- [x] Local and global logout semantics behave as specified and pass automated integration tests.

---

## Out of Scope

- Outbound webhook delivery worker (Phase 05 — outbox events emitted here will be delivered in Phase 05)
- Framework-specific TypeScript SDK packages (Phase 06)

