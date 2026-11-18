# Security Assessment & OWASP ASVS Level 2/3 Verification

This document records the adversarial security verification, cross-tenant red-team isolation audit, supply-chain hardening, and vulnerability remediations completed in Phase 10 for the Dradix Identity & Sync Platform.

---

## 1. Cross-Tenant Red-Team Isolation Matrix

Automated coverage in `apps/auth-api/tests/security_red_team_isolation_tests.rs` (unit + live PostgreSQL 16 integration suite executed in CI via `cargo test --locked --all-targets --all-features -- --ignored --test-threads=1`) models two independent developer workspaces, two applications (`Ferrum`, `Elysiar`), and four environments (`ferrum-dev`, `ferrum-prod`, `elysiar-dev`, `elysiar-prod`).

| Attack Vector | Enforced Boundary | Verification Test | Status |
| --- | --- | --- | --- |
| Cross-env user profile & metadata read/write | Composite `(environment_id, application_user_id)` queries across `backend_sync` & `tenant_memberships` | `test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation` | Verified |
| Cross-env `app_session` inspection & revocation | Environment-bound `app_sessions` lookups and revocations | `test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation` | Verified |
| Cross-application JWT `aud` substitution | ES256 signature verification enforces exact `aud == environment_id` and `iss == issuer_url` | `test_cross_application_jwt_aud_and_org_permission_isolation` + live suite | Verified |
| Cross-application `id_token` replay | `typ == "JWT"` and `aud == client_id` separation from `at+jwt` access tokens | `test_cross_application_jwt_aud_and_org_permission_isolation` | Verified |
| `sk_test_*` / `sk_live_*` cross-environment misuse | Prefix & environment kind matching (`key_matches_environment_kind`) + SHA-256 hash lookup scoped to environment | `test_cross_environment_credential_and_scope_escalation_isolation` | Verified |
| Secret key scope escalation | Explicit `TenantContext::require_scope` checks and strict `ALLOWED_SECRET_SCOPES` allowlist | `test_cross_environment_credential_and_scope_escalation_isolation` | Verified |
| Cross-workspace control-plane access | `require_application_access` / `require_workspace_access` RBAC checks on every `/v1/control/*` route | `test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation` | Verified |
| Cross-organization membership, invitation & role probes | Environment-scoped organization/invitation lookups, email-bound single-use tokens, `evaluate_scoped_permission` | `test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation` + `phase08_organizations_tests.rs` | Verified |
| Cross-application webhook fan-out & cursor probes | `dispatch_outbox_to_deliveries` filters endpoints strictly by `environment_id`; `list_events_catchup` rejects foreign `after_event_id` | `test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation` | Verified |
| Cross-environment analytics & audit isolation | `analytics_environment_rollups` keyed by `(environment_id, bucket_start)` | `test_live_two_workspaces_ferrum_elysiar_four_env_red_team_isolation` + `phase09_observability_tests.rs` | Verified |

---

## 2. OWASP ASVS Level 2/3 Verification Matrix

Automated regression coverage lives in `apps/auth-api/tests/security_owasp_asvs_tests.rs`, `apps/auth-api/tests/phase03_auth_engine_tests.rs`, `apps/auth-api/tests/phase04_oidc_sso_tests.rs`, `apps/auth-api/tests/phase05_sync_and_webhooks_tests.rs`, and `apps/auth-api/tests/phase09_observability_tests.rs`.

### 2.1 Authentication & Credential Hardening (ASVS V2)

- **Password Storage:** Argon2id (`Version::V0x13`) with per-password CSPRNG salts via `OsRng`.
- **Reactor Isolation & DoS Protection:** All Argon2id hashing and verification execute off Tokio async worker threads on a bounded blocking pool (`apps/auth-api/src/services/password_hasher.rs`, bounded by `ARGON2_MAX_CONCURRENCY`, clamped to `1..=32`, default `2..=8`).
- **Account Enumeration Resistance:** Unknown-email sign-in paths execute `verify_dummy_password_async` to equalize verification timing and return uniform invalid-credential errors.
- **Brute-Force & Lockout:** Per-IP rate limiting (`tower_governor` auth/sensitive tiers) and per-account Redis lockout tracking with safe fallback.
- **Constant-Time Verification:** Credential hashes, CSRF tokens, PKCE `S256` challenges, and webhook HMAC signatures use `subtle::ConstantTimeEq`.

### 2.2 Session, Cookie & CSRF Controls (ASVS V3 / V4)

- **Cookie Flags:** `session_token`, `refresh_token`, and `__Host-idp_sso_session` enforce `HttpOnly`, `SameSite=Lax`, and `Secure` when `COOKIE_SECURE=true`.
- **Refresh Token Family Protection:** Opaque `rt_*` tokens are hashed with SHA-256 at rest and rotated on every refresh; replay of a consumed token triggers family-wide revocation and increments `auth_refresh_token_reuse_detected_total`.
- **CSRF Protection:** Double-submit `csrf_token` cookie vs `x-csrf-token` header enforced by `csrf_protection_middleware` on cookie-authenticated mutating routes.

### 2.3 JWT, OIDC & Redirect Security (ASVS V2 / V3 / V5)

- **Algorithm Pinning:** Only `ES256` (ECDSA P-256) is accepted; `alg: none` and `HS256`-with-public-JWK confusion attacks are explicitly rejected (`test_jwt_and_oidc_attack_matrix`).
- **Type Separation:** Access tokens require `typ: "at+jwt"`; ID tokens require `typ: "JWT"`. Cross-type substitution fails verification.
- **Key Lifecycle:** Private signing keys and webhook secrets are envelope-encrypted with AES-256-GCM bound to purpose+version AAD (`SIGNING_KEY_AAD_PURPOSE`, `WEBHOOK_SECRET_AAD_PURPOSE`). Revoked `kid`s are immediately dropped from JWKS and rejected on verification (`test_live_emergency_signing_key_revocation_and_rotation_drill`).
- **PKCE Enforcement:** Discovery advertises only `S256`; `plain` downgrade and malformed verifiers are rejected.
- **Redirect URI & Origin Parser Differentials:** `validate_redirect_uri` and `validate_origin` reject userinfo (`https://good.com@evil.com`), path traversal (`/../`, `/./`), backslashes, fragments, wildcards, scheme-relative URLs (`//evil.com`), `javascript:`/`data:`, non-HTTPS production origins, localhost in production, and non-ASCII Unicode homograph hostnames (`test_redirect_uri_and_origin_parser_differential_attacks`).

### 2.4 Webhook SSRF & Delivery Integrity (ASVS V12 / V13)

- **Pre-Delivery URL Validation:** Rejects non-HTTPS production targets, embedded credentials, `.localhost`/`.local`/`.internal`/`metadata.google.internal`, and literal private/link-local/loopback IPs.
- **DNS-Time SSRF Rebinding Defense:** `SsrfSafeDnsResolver` inspects every resolved IPv4/IPv6 address prior to TCP connect and blocks `169.254.169.254`, RFC1918, CGNAT (`100.64.0.0/10`), loopback (`127.0.0.1`, `::1`), IPv6-mapped IPv4 (`::ffff:127.0.0.1`, `::ffff:169.254.169.254`), ULA (`fc00::/7`), and link-local (`fe80::/10`).
- **Redirect & Timeout Policy:** `reqwest::redirect::Policy::none()` blocks 3xx redirects; 3s connect timeout, 10s request timeout, and 4 KiB response body truncation (`MAX_RESPONSE_EXCERPT_BYTES`) defend against slow-loris and oversized payloads.
- **HMAC Replay Window:** `verify_signature_header` rejects signatures older or newer than `DEFAULT_SIGNATURE_TOLERANCE_SECS` (300s) and rejects any body mutation.

### 2.5 Web Headers, Logging & Supply-Chain Security (ASVS V7 / V14)

- **HTTP Security Headers:** `security_headers_middleware` enforces `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy`, `Strict-Transport-Security`, `Content-Security-Policy: default-src 'none'; frame-ancestors 'none'`, `COOP: same-origin`, `CORP: same-origin`, and strips `Server`.
- **Log Redaction:** `redact_sensitive_text` and `redact_sensitive_json` scrub passwords, OTPs, JWTs, `sk_*`, `whsec_*`, and `orginv_*` from structured JSON logs (`phase09_observability_tests.rs`).
- **Supply Chain & CI Gates:** `.github/workflows/ci.yml` and `.github/workflows/security.yml` enforce `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (unit + live Postgres/Redis integration), `cargo audit --deny unsound`, `npm audit --workspaces --audit-level=high`, `npm run check:vendor`, Gitleaks secret scanning, and Trivy container/filesystem scans.

---

## 3. Vulnerability & Remediation Log

| ID | Severity | Finding | Remediation | Verification |
| --- | --- | --- | --- | --- |
| SEC-P10-01 | High | Synchronous Argon2id password hashing/verification ran directly inside async handlers across `client_auth`, `oidc_provider`, `control_auth`, `control_ops`, and `authentication`, risking Tokio worker thread starvation under credential-stuffing spikes. | Introduced bounded `tokio::sync::Semaphore` + `tokio::task::spawn_blocking` pool in `apps/auth-api/src/services/password_hasher.rs` with `verify_dummy_password_async` timing parity and `argon2_hash_duration_seconds` instrumentation; migrated all 5 services. | `test_bounded_argon2id_pool_prevents_async_reactor_starvation` + `tests/load/auth_signin_saturation.js` |
| SEC-P10-02 | Medium | Live PostgreSQL `#[ignore]` integration tests in `phase08_organizations_tests.rs` and `phase09_observability_tests.rs` referenced stale schema names and `AppConfig::from_env()` without running migrations, which would fail CI database integration runs. | Standardized all live PostgreSQL integration tests to run `sqlx::migrate!("./migrations")`, use canonical schema tables (`developer_workspaces`, `developers.display_name`, `application_environments.slug`), and build `AppConfig` explicitly from `TEST_DATABASE_URL`. | `cargo clippy --all-targets` + CI `Run database integration tests` |

No critical or high unresolved security findings remain in the codebase.

