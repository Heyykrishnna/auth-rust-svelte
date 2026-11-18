# Comprehensive Security & Production Readiness Audit Report

> **Framework Applied:** `auditor-skill` v7.6 (Scope-Gated Multi-Language Security Audit Methodology — CertiK / EY SOC 2 / COBIT / OWASP Top 10:2025 / OWASP ASVS L2-L3)  
> **Target Repository:** `auth-rust-svelte` (Dradix Multi-Tenant Identity & Sync Platform)

---

## 1. Executive Summary

| Field | Value |
| --- | --- |
| **Repository** | `Heyykrishnna/auth-rust-svelte` |
| **Branch / Commit** | `main` @ `379fa6a` |
| **Audit Date** | `2026-11-18` |
| **Audit Scope** | `FULL` (Scope-Gated Off-Chain Rust + TypeScript/SvelteKit + DevOps/Supply Chain + Universal Checklists) |
| **Repository Risk Score** | **8 — HIGH** (Fix High finding before production deployment) |
| **Overall Production Score** | **74.0 / 100** |

### What We Found

We audited the complete `auth-rust-svelte` monorepo across its Rust Axum identity API (`apps/auth-api`, 105 `.rs` files / 37,938 LOC), SvelteKit Control Plane & Hosted Portal (`apps/web`, 44 `.svelte` + `.ts` files), four Edge-ready TypeScript SDKs (`packages/{core,svelte,react,next,shared-types}`), reference starters (`examples/{ferrum-sveltekit,elysiar-nextjs}`), 14 PostgreSQL migrations, Dockerfiles, Kubernetes manifests, observability stack, and GitHub Actions pipelines.

The platform demonstrates **strong foundational architecture**: zero `unsafe` Rust blocks, zero TypeScript `any` escape hatches or XSS sinks, strict ES256 JWT enforcement (`alg: none` and HS256-confusion resistant), AES-256-GCM envelope encryption, DNS-time SSRF webhook defenses, transactional-outbox delivery, a bounded Argon2id blocking pool, and 132 automated passing tests (62 Rust + 70 JS/TS/Web + 24 live-Postgres integration tests).

However, **the repository is NOT yet safe for unsupervised production deployment** due to **1 High finding (Severity 8)** and **11 Medium findings (Severity 5–6)**: most critically, `apps/auth-api/src/config/mod.rs` falls back to hardcoded development secrets for both `JWT_SECRET` and `SIGNING_KEY_KEK` when environment variables are absent, and `infrastructure/kubernetes/base/auth/secrets.yaml` omits `SIGNING_KEY_KEK` and `COOKIE_SECURE=true` — meaning a default Kubernetes rollout would silently boot production with a publicly committed encryption key and insecure cookies. Secondary pre-launch blockers include in-memory `PeerIpKeyExtractor` rate limiting (which collapses all users behind NGINX Ingress into a single bucket), `panic = "abort"` with no `CatchPanicLayer`, missing TOTP/WebAuthn MFA enforcement, soft-only GDPR deletion without PII anonymization, per-request `reqwest::Client` construction in webhook deliveries, and missing frontend CSP headers in SvelteKit SSR.

---

### 7-Pillar Production Readiness Scorecard (100 Points)

| Aspect | Score | Max | Grade | Key Drivers & Deductions |
| --- | ---: | ---: | --- | --- |
| **Security** | **22.0** | 30 | C+ | Strong multi-tenant isolation, SSRF, CSRF, ES256, zero XSS/`unsafe`; **-3** fail-open default secrets + K8s omission (`F-001`), **-2** proxy-broken in-memory rate limiter (`F-003`), **-1** non-zeroized secrets + `Debug` on `AppConfig` (`F-005`), **-1** missing SvelteKit SSR CSP/headers (`F-006`), **-1** floating GHA tags + unauthenticated `/metrics` (`F-008`) |
| **Authentication** | **15.0** | 20 | B- | Solid OIDC + PKCE S256, `__Host-` SSO cookie, refresh rotation + reuse revocation, bounded Argon2id + timing parity; **-2** no TOTP/WebAuthn MFA implementation despite `require_mfa` flag (`F-004`), **-1** sign-up/email enumeration (`F-007`), **-1** non-CT secondary email OTP + plaintext pending OTP in DB (`F-009`), **-1** weak password policy & missing max-length cap (`F-010`) |
| **Reliability** | **11.0** | 15 | C+ | Transactional outbox, `SKIP LOCKED` worker, circuit breaker, graceful `SIGTERM`, `/health` & `/ready`; **-1.5** `panic = "abort"` + no `CatchPanicLayer` + `.expect()` in OIDC path (`F-002`), **-1.5** unbounded fire-and-forget SMTP `tokio::spawn` swallowing errors (`F-011`), **-1** incomplete K8s Ingress routes & unexecuted live staging DR drills (`F-012`) |
| **Performance** | **11.0** | 15 | C+ | Bounded Argon2id pool, Rust LTO release profile, indexed Postgres schema, cursor pagination, k6 suite; **-1.5** `reqwest::Client` rebuilt per webhook/OAuth call (`F-013`), **-1.5** DB lookup + JWK parse on every JWT verification (`F-014`), **-1** live staging k6/soak baseline not yet executed (`F-015`) |
| **Privacy** | **6.0** | 10 | D+ | GDPR export & delete endpoints, webhook `user.deleted` fan-out, structured log PII/secret scrubbing; **-2** `delete_user_account` soft-deletes without anonymizing PII & no retention purge worker (`F-016`), **-1** no field-level PII encryption or formal PII inventory (`F-017`), **-1** missing Privacy Policy, ToS, and compliance mapping (`F-018`) |
| **Observability** | **4.5** | 5 | A- | End-to-end OTel `traceparent` across API→outbox→webhook, 20+ low-cardinality Prometheus metrics, 10 alert rules, 4 Grafana dashboards, analytics rollups & UI; **-0.5** `/metrics` on primary port `8080` + Alertmanager/90d immutable log sink not yet wired (`F-019`) |
| **Developer Experience** | **4.5** | 5 | A- | Clean monorepo, Edge-compliant SDKs, Control Plane UI, zero lint/type/fmt issues, 132 automated tests; **-0.5** missing `CODEOWNERS`, `CHANGELOG.md`, `SECURITY.md`, coverage & fuzzing in CI (`F-020`) |
| **TOTAL** | **74.0** | **100** | **C+** | **Conditional Pre-Launch — Fix `F-001`..`F-006` before production cutover** |

---

### Severity Distribution (`auditor-skill` 1–10 Scale)

| Score | Label | Count | Finding IDs |
| --- | --- | ---: | --- |
| 10 | CRITICAL | 0 | — |
| 9 | CRITICAL | 0 | — |
| 8 | HIGH | 1 | `F-001` |
| 7 | HIGH | 0 | — |
| 6 | MEDIUM | 3 | `F-002`, `F-003`, `F-004` |
| 5 | MEDIUM | 8 | `F-005`, `F-006`, `F-007`, `F-009`, `F-011`, `F-012`, `F-013`, `F-016` |
| 4 | LOW | 5 | `F-008`, `F-010`, `F-014`, `F-017`, `F-018` |
| 3 | LOW | 3 | `F-015`, `F-019`, `F-020` |
| 2 | INFO | 0 | — |
| 1 | INFO | 0 | — |
| **Total Findings** | | **20** | **1 High · 11 Medium · 8 Low · 0 Critical** |

### Items Verified (Scope-Gated Summary)

| Metric | Count |
| --- | ---: |
| Total `auditor-skill` corpus items | 1,446 |
| Out-of-scope items (on-chain Solana, Python, Go/Java/Ruby/PHP, Solana AI agents) | 736 |
| **In-scope checklist items (10 checklists)** | **710** |
| In-scope PASS | 318 |
| In-scope FAIL | 34 |
| In-scope PARTIAL | 61 |
| In-scope N/A (feature/domain absent within in-scope checklists, e.g. MongoDB/Solana sub-sections) | 297 |
| **In-scope Verdict Completion** | **100.0% (710 / 710)** |

---

## 2. Scope Coverage (Rule 0)

| Checklist / Vector Group | Status | Items | Trigger / Reason |
| --- | --- | ---: | --- |
| 01 — Account Validation | `OUT-OF-SCOPE` | 0 / 90 | No `Anchor.toml` or `programs/` on-chain Solana code |
| 02 — Access Control (On-Chain) | `OUT-OF-SCOPE` | 0 / 55 | No Solana on-chain program |
| 03 — Arithmetic Safety (On-Chain) | `OUT-OF-SCOPE` | 0 / 63 | No Solana on-chain program |
| 04 — CPI & PDA Safety | `OUT-OF-SCOPE` | 0 / 70 | No Solana on-chain program |
| 05 — State Machine (On-Chain) | `OUT-OF-SCOPE` | 0 / 74 | No Solana on-chain program |
| 06 — Economic & Logic (DeFi) | `OUT-OF-SCOPE` | 0 / 92 | No Solana DeFi protocol |
| 07 — OpSec & Governance (On-Chain) | `OUT-OF-SCOPE` | 0 / 87 | No on-chain program authority |
| **08 — TypeScript Safety** | **`IN-SCOPE`** | **64 / 64** | 63 `.ts` files across `apps/web`, `packages/*`, `examples/*` |
| **09 — Backend Security** | **`IN-SCOPE`** | **132 / 132** | `apps/auth-api` HTTP/OIDC/Backend API + SvelteKit SSR |
| **10 — Frontend Security** | **`IN-SCOPE`** | **84 / 84** | `apps/web` SvelteKit UI + React/Next/Svelte SDKs |
| **11 — Supply Chain & Dependencies** | **`IN-SCOPE`** | **62 / 62** | Universal (`Cargo.lock`, `package-lock.json`, `vendor/picomatch`) |
| **12 — Secrets & Key Management** | **`IN-SCOPE`** | **53 / 53** | Universal (`AppConfig`, `signing_keys`, `.env`, K8s secrets) |
| **13 — Deployment & Infrastructure** | **`IN-SCOPE`** | **96 / 96** | Universal (Dockerfiles, K8s, GitHub Actions) |
| 14 — Python Safety | `OUT-OF-SCOPE` | 0 / 82 | No Python application files |
| 15 — General Language Safety | `OUT-OF-SCOPE` | 0 / 88 | No Go/Java/Ruby/PHP application files |
| **16 — Formal Verification & Testing** | **`IN-SCOPE`** | **73 / 73** | Universal (Rust tests, Vitest, SDK tests, CI) |
| **17 — Logging, Monitoring & IR** | **`IN-SCOPE`** | **65 / 65** | Universal (OTel, Prometheus, Grafana, DR runbooks) |
| **18 — Privacy, Compliance & Change Mgmt** | **`IN-SCOPE`** | **60 / 60** | Universal (GDPR export/delete, PII, SDLC, CI/CD) |
| 19 — AI Agent Security (Solana × AI) | `OUT-OF-SCOPE` | 0 / 35 | No `.mcp.json` or LLM/agent runtime in application code |
| **20 — Rust Off-Chain Services** | **`IN-SCOPE`** | **21 / 21** | 105 `.rs` files in `apps/auth-api` (outside `programs/`) |
| **Known Vectors (KV-001..KV-140)** | **Scope-Gated** | **140 / 140** | 75 in-scope off-chain/universal evaluated; 65 on-chain/AI gated `OUT-OF-SCOPE` |

---

## 3. Scope & Methodology

### Codebase Inventory (Commit `379fa6a`)

| Extension | Files | Lines of Code | Primary Locations |
| --- | ---: | ---: | --- |
| `.rs` | 105 | 37,938 | `apps/auth-api/src/`, `apps/auth-api/tests/` |
| `.svelte` | 44 | 11,693 | `apps/web/src/`, `examples/ferrum-sveltekit/` |
| `.ts` | 63 | 9,276 | `apps/web/src/`, `packages/*`, `examples/*` |
| `.yaml` / `.yml` | 117 | 3,712 | `infrastructure/kubernetes/`, `.github/workflows/`, `observability/`, `docker-compose*.yml` |
| `.mjs` / `.js` | 10 | 2,425 | `tests/load/`, SDK test suites, config |
| `.sql` | 14 | 1,138 | `apps/auth-api/migrations/001..014` |

### Operating Safety Disclosure (Rule 11a & 11b)

- **Execution Mode (Rule 11a):** Verification commands (`cargo fmt --check`, `cargo clippy`, `cargo test`, `npm run lint`, `npm run check`, `npm test`, `npm run build`) ran in **native** mode (sandboxed where permitted; `npm run check:vendor` and `npm audit` executed outside the sandbox on the host). No untrusted remote code was fetched or executed. Static git history scans used `GIT_NO_LAZY_FETCH=1 git --no-pager -c core.fsmonitor=false -c log.showSignature=false`.
- **Outside Services Contacted (Rule 11b):** `npm audit --workspaces --audit-level=high` queried the public npm advisory endpoint (`registry.npmjs.org`) sending workspace package names and resolved versions. No source code, secrets, or proprietary payloads were transmitted. `cargo-audit` is not installed locally; Cargo advisory scanning is enforced in CI (`.github/workflows/security.yml`).

---

## 4. Detailed Findings (Severity ≥ 4 — Rule 5 & Rule 5b)

---

#### [F-001] Fail-Open Hardcoded Fallback Secrets for `JWT_SECRET` and `SIGNING_KEY_KEK` Compounded by Missing Kubernetes Secret Keys

| Field | Value |
| --- | --- |
| **Severity** | **8 — HIGH** |
| **Checklist Items** | `SEC-013`, `BE-066`, `RS-017`, `KV-054` |
| **Pillar** | Security (-3 pts) |
| **Category** | Secrets & Key Management / Cryptography |
| **Language** | Rust + Kubernetes YAML |
| **Files** | `apps/auth-api/src/config/mod.rs:69-90`, `infrastructure/kubernetes/base/auth/secrets.yaml:13-20`, `infrastructure/kubernetes/base/auth/auth-api/deployment.yaml:41-54` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`AppConfig::from_env()` silently falls back to hardcoded, publicly committed placeholder strings when `JWT_SECRET` (`"dev_jwt_secret_change_me_in_production_min_32_chars"`, line 70) or `SIGNING_KEY_KEK` (`"dev_signing_key_kek_change_me_in_production_32_bytes_min"`, line 84) is omitted from the environment. Simultaneously, `infrastructure/kubernetes/base/auth/secrets.yaml` omits `SIGNING_KEY_KEK`, `SIGNING_KEY_KEK_VERSION`, `COOKIE_SECURE`, and `ISSUER_URL` entirely, and `deployment.yaml` sets none of them.

**Rule 5b Validation Gate:**
- **Reachability:**
  - Entry point: `apps/auth-api/src/main.rs:14` → `AppConfig::from_env()` (`config/mod.rs:44-91`) → `signing_keys::encrypt_envelope()` (`signing_keys.rs:54-81`).
  - Signer / authority required: None (deployment misconfiguration / default K8s manifest apply).
  - Preconditions: Deploy `auth-api` using the repository's K8s base/prod manifests without manually adding `SIGNING_KEY_KEK`.
  - Guard analysis: `config/mod.rs:72,86` only checks `.len() >= 32`; both hardcoded defaults exceed 32 characters, so startup succeeds silently.
  - Verdict: **REACHABLE**
- **Math / State-Bounds:**
  - Vulnerable transition: `SIGNING_KEY_KEK` missing → `derive_kek_bytes("dev_signing_key_kek_change_me_in_production_32_bytes_min")` (`signing_keys.rs:45-52`) encrypts every ES256 private signing key, every tenant OAuth client secret, and every webhook signing secret under a publicly known SHA-256 key.
  - Net effect: Any actor with read access to a DB backup/snapshot/SQL leak can decrypt **100% of platform ES256 private keys** and forge arbitrary access/ID tokens for every workspace, application, and environment, plus forge arbitrary signed webhooks.
- **Attacker-Model:**
  - Capability: Passive read of any DB snapshot/backup or compromised read-only replica + knowledge of public repository source.
  - Capital / setup cost: Zero.
  - Profit / damage: Complete multi-tenant identity compromise across all customer applications.
  - Atomicity: Offline decryption once ciphertext is obtained.
  - Net: **Critical blast radius bounded to DB-read precondition → reported at Severity 8 (HIGH)**.

**Impact:**
Silent cryptographic collapse of envelope encryption (`signing_keys`, `webhook_endpoints`, `application_oauth_providers`) and insecure cookie flags (`cookie_secure = false`) on any deployment missing those env vars.

**Recommendation:**
```rust
let is_production = env::var("APP_ENV")
    .or_else(|_| env::var("RUST_ENV"))
    .map(|v| v.eq_ignore_ascii_case("production"))
    .unwrap_or(false)
    || cookie_secure;

let signing_key_kek = env::var("SIGNING_KEY_KEK")
    .context("SIGNING_KEY_KEK must be explicitly set (no fallback allowed)")?;
let jwt_secret = env::var("JWT_SECRET")
    .context("JWT_SECRET must be explicitly set (no fallback allowed)")?;
if is_production && (signing_key_kek.contains("change_me") || jwt_secret.contains("change_me")) {
    anyhow::bail!("Refusing to boot in production with placeholder secrets");
}
```
Also add `SIGNING_KEY_KEK`, `SIGNING_KEY_KEK_VERSION`, `COOKIE_SECURE`, and `ISSUER_URL` to `infrastructure/kubernetes/base/auth/secrets.yaml` and `externalsecret-auth.yaml`.

---

#### [F-002] Process-Aborting Panics in Release Profile Without `CatchPanicLayer` and Reachable `.expect()` Calls

| Field | Value |
| --- | --- |
| **Severity** | **6 — MEDIUM** |
| **Checklist Items** | `RS-001`, `RS-005`, `FV-049` |
| **Pillar** | Reliability (-1.5 pts) |
| **Category** | Error Handling & DoS Resilience |
| **Language** | Rust |
| **Files** | `apps/auth-api/Cargo.toml:99`, `apps/auth-api/src/routes/mod.rs:29-90`, `apps/auth-api/src/services/oidc_provider.rs:780,795` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`[profile.release]` sets `panic = "abort"` (`Cargo.toml:99`), `build_router` (`routes/mod.rs:29-90`) installs no `tower_http::catch_panic::CatchPanicLayer`, and `oidc_provider.rs:780,795` unwraps `active_sso` via `.expect(...)` inside request handling. Any future panic in a handler, middleware, or transitive dependency immediately terminates the entire `auth-api` process and drops all concurrent in-flight requests on that pod.

**Rule 5b Validation Gate:**
- **Reachability:** Entry point `GET /oauth2/authorize` (`routes/oidc.rs:10`) → `oidc_provider.rs:780,795` (currently guarded by `evaluate_sso_policy` invariant, but zero panic boundary exists across all 80+ HTTP handlers). Verdict: **REACHABLE** (architectural single-point-of-failure at request boundary).
- **Math / State-Bounds:** With `panic = "abort"`, `1` panicked task = `100%` of in-flight HTTP requests on the pod dropped instantly (no unwind, no graceful drain).

**Recommendation:**
1. Change `panic = "unwind"` in `apps/auth-api/Cargo.toml` `[profile.release]` and enable `tower-http`'s `catch-panic` feature, adding `CatchPanicLayer::new()` in `routes/mod.rs`.
2. Replace `active_sso.expect(...)` at `oidc_provider.rs:780,795` with `.ok_or_else(|| AppError::Internal(...))?`.

---

#### [F-003] In-Memory `PeerIpKeyExtractor` Rate Limiting Collapses Behind Reverse Proxies & Fails Across Replicas

| Field | Value |
| --- | --- |
| **Severity** | **6 — MEDIUM** |
| **Checklist Items** | `BE-034`, `BE-035`, `KV-039`, `KV-083` |
| **Pillar** | Security (-2 pts) |
| **Category** | Rate Limiting / Availability & Abuse Prevention |
| **Language** | Rust + Kubernetes |
| **Files** | `apps/auth-api/src/middleware/rate_limit.rs:8-48`, `infrastructure/kubernetes/base/auth/auth-api/deployment.yaml:10` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`rate_limit.rs` uses in-memory `tower_governor` keyed by `PeerIpKeyExtractor` (`per_second(2), burst_size(5)` for auth; `per_second(1), burst_size(3)` for sensitive). In Kubernetes (`deployment.yaml:10`, `replicas: 3`) behind NGINX Ingress (`ingress.yaml`), every HTTP request arrives with the NGINX Ingress pod's TCP peer IP. Consequently:
1. All global end users share a single 2 req/s auth bucket per pod — 5 concurrent sign-ins from unrelated users trigger global `429 Too Many Requests` self-DoS.
2. Across `N` replicas, an attacker hitting different pods multiplies effective rate limit by `N` (no shared Redis store).

**Rule 5b Validation Gate:**
- **Reachability:** Every `/v1/client/auth/*`, `/oauth2/*`, and `/v1/control/auth/*` request passes through `auth_rate_limit_layer()` / `sensitive_rate_limit_layer()`. Verdict: **REACHABLE**.
- **Math / State-Bounds:** Under NGINX Ingress, `distinct_peer_ips == ingress_pod_count` (typically 1–2). At 6 login requests/sec platform-wide across all tenants, `> 66%` of legitimate sign-ins receive `429`.

**Recommendation:**
Implement a Redis-backed sliding-window rate limiter keyed on trusted client IP (parsed from the right-most untrusted hop of `X-Forwarded-For` when `TRUST_PROXY_HOPS > 0`, never blindly trusting the left-most spoofable header) plus per-environment / per-secret-key quotas (`BE-116`).

---

#### [F-004] MFA (`TOTP` / `WebAuthn`) Policy Flag Exposed Without Enrollment or Step-Up Verification Endpoints

| Field | Value |
| --- | --- |
| **Severity** | **6 — MEDIUM** |
| **Checklist Items** | `BE-102`, `KV-126` |
| **Pillar** | Authentication (-2 pts) |
| **Category** | Multi-Factor Authentication & Step-Up Authorization |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/oidc_provider.rs:389`, `apps/auth-api/src/services/control_plane.rs:79` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
Environment configuration exposes `require_mfa: bool` and `oidc_provider.rs:389` checks `amr` for `"mfa" | "totp" | "webauthn" | "otp"`, yet `auth-api` implements no TOTP (`RFC 6238`) secret enrollment/verification, no WebAuthn/Passkey registration/assertion, and no backup recovery codes. Enabling `require_mfa` locks out password users who cannot satisfy an MFA challenge, and developer control-plane actions (creating/rotating secret keys, adding webhooks) have no step-up MFA option.

**Rule 5b Validation Gate:**
- **Reachability:** Control-plane admin sets `require_mfa: true` via `PATCH /v1/control/applications/:id/environments/:env` → `evaluate_sso_policy` rejects password-only sessions with no enrollment flow to satisfy it; meanwhile developer control-plane mutations rely solely on password session cookies. Verdict: **REACHABLE**.
- **Math / State-Bounds:** Single-factor password compromise gives full control-plane access to rotate/create `sk_live_*` keys and register exfiltration webhooks.

**Recommendation:**
Implement TOTP (`RFC 6238`) + encrypted recovery codes (and optionally WebAuthn/Passkeys) with a step-up challenge for `require_mfa` environments and sensitive control-plane credential/webhook mutations.

---

#### [F-005] In-Memory Cryptographic Secrets Not Zeroized & Plaintext Secret Exposure via `#[derive(Debug)]` on `AppConfig`

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `RS-015`, `RS-016`, `KV-112` |
| **Pillar** | Security (-1 pt) |
| **Category** | Cryptography / Memory Hygiene |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/config/mod.rs:6-41`, `apps/auth-api/src/services/signing_keys.rs:83-115,223-233`, `apps/auth-api/src/services/webhooks.rs:409-442` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
1. `AppConfig` derives `Debug` (`config/mod.rs:6`), printing plaintext `jwt_secret`, `signing_key_kek`, `google_client_secret`, `github_client_secret`, `smtp_pass`, and `database_url` if `AppConfig` is ever logged with `{:?}`.
2. Decrypted ES256 PKCS#8 DER private keys (`Vec<u8>`) and webhook signing secrets (`String`) are plain heap allocations never zeroized on drop (`zeroize` is absent from `apps/auth-api/Cargo.toml`).

**Recommendation:**
Implement a custom `fmt::Debug` for `AppConfig` redacting all secret fields (`"[REDACTED]"`), and wrap decrypted PKCS#8 keys, KEK bytes, and webhook secrets in `zeroize::Zeroizing<Vec<u8>>` / `Zeroizing<String>`.

---

#### [F-006] Missing Security Headers & Content Security Policy in SvelteKit SSR (`apps/web/src/hooks.server.ts`)

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `FE-066`, `FE-070`, `KV-059`, `KV-071`, `KV-096` |
| **Pillar** | Security (-1 pt) |
| **Category** | Frontend / Clickjacking & XSS Defense-in-Depth |
| **Language** | TypeScript (SvelteKit) |
| **Files** | `apps/web/src/hooks.server.ts:84` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
While `auth-api` attaches full security headers via `security_headers_middleware` and `infrastructure/kubernetes/base/auth/ingress.yaml:31-41` injects headers at the NGINX layer, `apps/web/src/hooks.server.ts` returns `resolve(event)` directly without setting `Content-Security-Policy`, `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy`, or `Strict-Transport-Security`. Any deployment not fronted by that exact NGINX snippet (Docker Compose, Fly.io, Cloud Run, Vercel, Render) serves the Hosted Login Portal and Control Plane Dashboard without clickjacking or CSP protection.

**Recommendation:**
Configure SvelteKit's built-in nonce-based `csp` in `svelte.config.js` and attach `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`, and `Permissions-Policy` in `apps/web/src/hooks.server.ts` after `resolve(event)`.

---

#### [F-007] Account Enumeration via Distinct Conflict Responses on Registration & Email Linking

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `BE-099`, `KV-051` |
| **Pillar** | Authentication (-1 pt) |
| **Category** | Authentication / Information Disclosure |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/client_auth.rs:249-253`, `apps/auth-api/src/services/identity_advanced.rs:170-174` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
Although `sign_in` properly uses `verify_dummy_password_async` to prevent login enumeration, `initiate_sign_up` (`client_auth.rs:249-253`) returns `409 Conflict (EMAIL_ALREADY_EXISTS)` when an email is already registered. Similarly, `add_user_email` (`identity_advanced.rs:170-174`) confirms whether an arbitrary email is already verified on another user's account.

**Recommendation:**
Return a uniform `200/202` "If this email is eligible, a verification code has been sent" response on `initiate_sign_up` (sending an informational notice to existing accounts instead of leaking existence), or make enumeration protection a configurable per-environment toggle defaulting to `true` in production.

---

#### [F-009] Non-Constant-Time Secondary Email OTP Comparison, Missing Attempt Cap & Plaintext Pending OTP Storage

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `BE-099`, `SEC-053` |
| **Pillar** | Authentication (-1 pt) |
| **Category** | Cryptographic Verification & Brute-Force Protection |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/identity_advanced.rs:189-227`, `apps/auth-api/src/services/client_auth.rs:257-265` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
1. `verify_user_email` (`identity_advanced.rs:219`) compares the 6-digit Redis OTP using non-constant-time `expected == code.trim()` and tracks zero failed verification attempts (allowing up to 10 minutes of repeated guesses limited only by IP rate limit).
2. `initiate_sign_up` (`client_auth.rs:259-265`) stores the 6-digit `code` in plaintext inside `PendingClientSignUp` JSON in `authentication_transactions.scope` instead of storing `hash_token(&code)`.

**Recommendation:**
Store only SHA-256 hashes of OTPs (both in Redis and `authentication_transactions.scope`), verify with `subtle::ConstantTimeEq`, and enforce a strict `max_attempts = 5` counter that invalidates the OTP upon exhaustion.

---

#### [F-011] Unbounded Fire-and-Forget `tokio::spawn` for SMTP Delivery Silently Discarding Failures

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `RS-013`, `FV-054` |
| **Pillar** | Reliability (-1.5 pts) |
| **Category** | Async Backpressure & Delivery Reliability |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/client_auth.rs:299,597,1313,1689`, `apps/auth-api/src/services/authentication.rs:72,235,646` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
Seven transactional email paths (OTP verification, resend OTP, password reset) spawn unbounded `tokio::spawn` tasks and discard the return `Result` with `let _ = send_branded_*_email(...).await;`. Under an OTP-flood or slow SMTP server, unbounded tasks accumulate without backpressure, SMTP failures are completely invisible to metrics/alerts, and graceful `SIGTERM` shutdown can terminate mid-SMTP handshake.

**Recommendation:**
Route outbound transactional emails through a bounded `tokio::sync::mpsc::channel(N)` worker (or reuse `outbox_events` / dedicated email outbox table) with structured error logging, Prometheus failure counter (`auth_email_deliveries_total`), and retry backoff.

---

#### [F-012] Kubernetes Ingress Missing Multi-Tenant Routes (`/v1/*`, `/oauth2/*`, `/.well-known/*`) & Live Staging DR Drills Unexecuted

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `DEP-049`, `LM-054`, `LM-059`, `KV-100` |
| **Pillar** | Reliability (-1 pt) |
| **Category** | Deployment Configuration & Disaster Recovery Verification |
| **Language** | Kubernetes YAML / Operations |
| **Files** | `infrastructure/kubernetes/base/auth/ingress.yaml:51-72`, `docs/disaster-recovery.md` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
1. `ingress.yaml` only forwards `/`, `/auth`, and `/health` — omitting `/v1`, `/oauth2`, `/.well-known`, and `/api`, so the multi-tenant client/backend/control planes and OIDC discovery/JWKS endpoints would 404 through the SvelteKit fallback in K8s.
2. While the emergency signing-key revocation drill is automated against PostgreSQL 16 (`security_owasp_asvs_tests.rs`), live staging execution records for PostgreSQL PITR restore, Redis outage failover, and `webhook-worker` `SIGKILL` recovery have not yet been captured (scheduled for Phase 11 cloud rollout).

**Recommendation:**
Add `/v1`, `/oauth2`, `/.well-known`, and `/api` path entries to `infrastructure/kubernetes/base/auth/ingress.yaml` (pointing to `auth-api`), and record dated staging execution logs for PITR restore and chaos drills prior to Phase 11 cutover.

---

#### [F-013] Per-Delivery `reqwest::Client` Construction Defeats Connection Pooling & Re-Initializes TLS Roots

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `RS-013`, `FV-051` |
| **Pillar** | Performance (-1.5 pts) |
| **Category** | Resource Efficiency & Throughput |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/webhooks.rs:537`, `apps/auth-api/src/services/social_oauth.rs:382,466` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`execute_single_delivery` calls `build_ssrf_safe_http_client(allow_loopback)?` on every single webhook delivery attempt, and `exchange_google_code` / `exchange_github_code` build a fresh `reqwest::Client` on every OAuth login. Each `reqwest::Client::build()` allocates a new TLS root certificate store and connection pool, defeating TCP/TLS keep-alive reuse and significantly degrading throughput during webhook fan-out bursts.

**Recommendation:**
Initialize two shared `OnceLock<reqwest::Client>` instances in `services/webhooks.rs` (one for `allow_loopback = false`, one for `allow_loopback = true`) and one shared `OnceLock<reqwest::Client>` in `services/social_oauth.rs`.

---

#### [F-016] GDPR Account Deletion Soft-Deletes Without PII Anonymization or Automated Retention Purge

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `PC-008`, `PC-009` |
| **Pillar** | Privacy (-2 pts) |
| **Category** | Data Privacy & GDPR Right to Erasure |
| **Language** | Rust / PostgreSQL |
| **Files** | `apps/auth-api/src/services/identity_advanced.rs:680-761` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`delete_user_account` updates `users.status = 'deleted'`, revokes sessions, and updates `application_users.status = 'revoked'`, but leaves plaintext `users.email`, `users.display_name`, `users.avatar_url`, `user_emails.email`, `oauth_accounts.email`, and `audit_logs` IP/user-agent records permanently stored in PostgreSQL with no tombstoning or scheduled retention-purge job.

**Recommendation:**
On `delete_user_account`, immediately anonymize PII columns (`email = 'deleted-' || id || '@erased.invalid'`, `display_name = 'Deleted User'`, `avatar_url = NULL`, `password_hash = NULL`, `DELETE FROM user_emails / oauth_accounts`), and add a scheduled retention job pruning expired `audit_logs` / `outbox_events` / `webhook_deliveries` past the configured retention window (e.g., 90–365 days).

---

#### [F-008] Floating Tag Pins in GitHub Actions Workflows & Unauthenticated `/metrics` on Primary API Port

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `DEP-067`, `KV-055`, `KV-082` |
| **Pillar** | Security (-1 pt) |
| **Category** | CI/CD Supply Chain & Endpoint Surface |
| **Language** | YAML / Rust |
| **Files** | `.github/workflows/ci.yml`, `.github/workflows/security.yml`, `.github/workflows/release.yml`, `apps/auth-api/src/routes/mod.rs:58-68` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
1. All GitHub Actions workflows pin third-party actions (`actions/checkout@v7`, `gitleaks/gitleaks-action@v2`, `aquasecurity/trivy-action@v0.36.0`, `imranismail/setup-kustomize@v2`, `Swatinem/rust-cache@v2`) by mutable tags rather than immutable 40-char commit SHAs + `# vX.Y.Z` comments, and no `.github/dependabot.yml` updates them (`DEP-067`).
2. `auth-api` mounts `GET /metrics` unauthenticated on the same public port `8080` as end-user traffic (`routes/mod.rs:58-68`). While K8s `ingress.yaml` does not route `/metrics` externally, any reverse proxy forwarding `/*` to `auth-api:8080` exposes internal operational metrics publicly.

**Recommendation:**
Pin all GitHub Actions to full 40-hex commit SHAs with `# vX.Y.Z` comments and add `.github/dependabot.yml` for `github-actions`, `cargo`, and `npm`. Move `auth-api`'s `/metrics` endpoint onto a dedicated internal port (`METRICS_PORT=9090`, matching `webhook-worker`'s `9091`) or require a bearer `METRICS_SCRAPE_TOKEN`.

---

#### [F-010] Missing Password Max-Length Bound Before Argon2id & Basic Minimum Length Policy

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `KV-094`, `BE-095` |
| **Pillar** | Authentication (-1 pt) |
| **Category** | Input Bounds & Credential Policy |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/client_auth.rs:233,612`, `apps/auth-api/src/services/password_hasher.rs:47-88` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`initiate_sign_up` enforces only `len >= 8` with no upper bound on password length in `hash_password_async` / `verify_password_async` (bounded only by the 64 KiB HTTP body limit). A 60 KiB password increases Argon2id pre-hashing overhead. NIST SP 800-63B also recommends rejecting passwords `> 128` bytes and checking breached-password lists.

**Recommendation:**
Enforce `password.len() <= 128` in `password_hasher.rs` / request validators (returning `422 ValidationError` cleanly) and consider optional minimum length `>= 10–12` or k-anonymity breached-password screening.

---

#### [F-014] Uncached PostgreSQL Lookup & JWK Decoding on Every Authenticated Request

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `BE-062` |
| **Pillar** | Performance (-1.5 pts) |
| **Category** | Token Verification Latency & DB Load |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/tokens.rs:291-299,434-446` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
Every call to `verify_tenant_access_token` and `verify_any_tenant_access_token` queries PostgreSQL `signing_keys` by `kid`, deserializes `public_jwk`, and constructs a `DecodingKey`. Under high-RPS authenticated traffic, this adds an avoidable DB round-trip per request.

**Recommendation:**
Cache verifiable `kid → DecodingKey` entries in a bounded in-memory TTL cache (e.g., 30–60s TTL, explicitly invalidated on `rotate_signing_key` / `revoke_signing_key` via Redis pub/sub or short TTL).

---

#### [F-017] No Field-Level PII Encryption at Rest or Formal Data Inventory

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `PC-001`, `PC-002`, `PC-006` |
| **Pillar** | Privacy (-1 pt) |
| **Category** | Data Protection & Governance |
| **Language** | SQL / Documentation |
| **Files** | `apps/auth-api/migrations/001_initial_schema.sql`, `apps/auth-api/migrations/007_multi_tenant_control_plane.sql` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
PII columns (`users.email`, `users.display_name`, `user_emails.email`, `audit_logs.ip_address`) rely solely on infrastructure volume encryption rather than documented cloud KMS/TDE controls or application-level blind-index encryption, and there is no formal PII inventory documenting purpose, lawful basis, and retention per column.

**Recommendation:**
Add `docs/privacy-and-data-inventory.md` mapping every PII column, purpose, retention window, and infrastructure encryption-at-rest control (Cloud SQL / RDS KMS CMK) required in Phase 11.

---

#### [F-018] Missing Privacy Policy, Terms of Service & Regulatory Compliance Mapping

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `PC-005`, `PC-015`, `PC-016`, `PC-018`, `PC-024` |
| **Pillar** | Privacy (-1 pt) |
| **Category** | Governance & Legal Compliance |
| **Language** | Documentation |
| **Files** | Repository root / `docs/` (absent) |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
No Privacy Policy, Terms of Service, or SOC 2 / GDPR control-mapping document exists in the repository for the Hosted Auth Portal or Developer Control Plane.

**Recommendation:**
Add Privacy Policy & ToS templates linked from the Hosted Auth Portal (`/portal/*`) and Control Plane sign-up flows, plus a SOC 2 / GDPR control matrix before public launch (Phase 12).

---

### Low-Severity Findings (Severity 3 — Tracked Inline)

- **`[F-015]` (Sev 3 — LOW | `FV-044` | Performance -1 pt):** `tests/load/*.js` k6 load/soak scripts are syntactically verified and thresholded, but live staging benchmark runs (p95/p99 latency, peak RSS, `db_pool_connections_in_use` over 2h soak) have not yet been executed and recorded in `docs/performance-report.md`.
- **`[F-019]` (Sev 3 — LOW | `LM-020`, `LM-021`, `LM-032` | Observability -0.5 pt):** Prometheus alerts exist in `observability/prometheus/alerts.yml`, but production Alertmanager receivers (PagerDuty/Slack escalation), tamper-evident 90-day log retention sink, and TLS cert expiry monitoring are not yet provisioned (Phase 11 target).
- **`[F-020]` (Sev 3 — LOW | `FV-023`, `FV-033`, `LM-051`, `PC-029`, `PC-036` | Developer Experience -0.5 pt):** Missing `.github/CODEOWNERS`, `CHANGELOG.md`, root `LICENSE`, `SECURITY.md` disclosure policy, CI test coverage reporting (`cargo-llvm-cov` / Vitest coverage), and fuzz/property-based tests (`cargo-fuzz` / `proptest`) for JWT/URL/webhook parsers.

---

## 5. Detailed Per-Checklist Summary & Verdicts (Rule 4 & Rule 8)

### Per-Checklist Summary Table

| # | Checklist | Scope | Items | Pass | Fail | Partial | N/A | Pass Rate (excl. N/A) |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 01–07 | On-Chain Solana (7 checklists) | OUT-OF-SCOPE | 531 | 0 | 0 | 0 | 531 | N/A |
| 08 | TypeScript Safety | IN-SCOPE | 64 | 31 | 0 | 3 | 30 | **91.2%** |
| 09 | Backend Security | IN-SCOPE | 132 | 61 | 8 | 10 | 53 | **77.2%** |
| 10 | Frontend Security | IN-SCOPE | 84 | 38 | 3 | 5 | 38 | **82.6%** |
| 11 | Supply Chain & Dependencies | IN-SCOPE | 62 | 28 | 1 | 5 | 28 | **82.4%** |
| 12 | Secrets & Key Management | IN-SCOPE | 53 | 22 | 3 | 6 | 22 | **71.0%** |
| 13 | Deployment & Infrastructure | IN-SCOPE | 96 | 30 | 5 | 9 | 52 | **68.2%** |
| 14 | Python Safety | OUT-OF-SCOPE | 82 | 0 | 0 | 0 | 82 | N/A |
| 15 | General Language Safety | OUT-OF-SCOPE | 88 | 0 | 0 | 0 | 88 | N/A |
| 16 | Formal Verification & Testing | IN-SCOPE | 73 | 26 | 4 | 6 | 37 | **72.2%** |
| 17 | Logging, Monitoring & IR | IN-SCOPE | 65 | 37 | 2 | 7 | 19 | **80.4%** |
| 18 | Privacy, Compliance & Change Mgmt | IN-SCOPE | 60 | 26 | 5 | 8 | 21 | **66.7%** |
| 19 | AI Agent Security | OUT-OF-SCOPE | 35 | 0 | 0 | 0 | 35 | N/A |
| 20 | Rust Off-Chain Services | IN-SCOPE | 21 | 11 | 3 | 2 | 5 | **68.8%** |
| **Total In-Scope** | **10 In-Scope Checklists** | **100% Evaluated** | **710** | **318** | **34** | **61** | **297** | **77.0%** |

*(Note: Individual checklist FAIL counts sum to 34 because shared root causes like `F-001` or `F-003` map across multiple checklists; de-duplicated unique findings = 20.)*

### Key Item-Level Verdicts by In-Scope Checklist

#### Checklist 20 — Rust Off-Chain Services (21 items)
- `[PASS]` `RS-002`: Zero `panic!`, `todo!`, `unimplemented!`, `unreachable!` in `apps/auth-api/src/` (verified by full AST/regex scan).
- `[PASS]` `RS-003`: No unchecked slice indexing on attacker-controlled lengths; `webhooks.rs:398-407` uses char-boundary safe truncation.
- `[PASS]` `RS-004`: No attacker-controlled integer division/remainder without bounds/clamping.
- `[PASS]` `RS-006`: Zero `std::sync::Mutex`/`RwLock` poisoning hazards (`std_sync_lock: 0`).
- `[PASS]` `RS-007..009`: Zero `unsafe` blocks (`unsafe_kw: 0`) across 37,938 LOC of Rust.
- `[PASS]` `RS-010..011`: `overflow-checks = true` explicitly enabled in `apps/auth-api/Cargo.toml:101` `[profile.release]`.
- `[PASS]` `RS-012`: No blocking std locks held across `.await`; Argon2id offloaded to `spawn_blocking` via `password_hasher.rs`.
- `[PASS]` `RS-014`: Timeouts configured on HTTP router (`TimeoutLayer(30s)`), SQLx pool (`acquire_timeout(30s)`), webhook client (`connect_timeout(3s)`, `timeout(10s)`), and OAuth client (`timeout(10s)`).
- `[FAIL-6]` `RS-001`, `RS-005`: `panic = "abort"` (`Cargo.toml:99`), no `CatchPanicLayer` (`routes/mod.rs`), and two `.expect(...)` calls (`oidc_provider.rs:780,795`) — see `[F-002]`.
- `[FAIL-5]` `RS-013`: Unbounded `tokio::spawn` for SMTP delivery (`client_auth.rs`, `authentication.rs`) — see `[F-011]`.
- `[FAIL-5]` `RS-015`, `[PARTIAL]` `RS-016`: Decrypted ES256 private keys & webhook secrets not wrapped in `Zeroizing<...>`; `AppConfig` derives `Debug` with plaintext secrets — see `[F-005]`.
- `[FAIL-8]` `RS-017`: `AppConfig::from_env()` falls back to hardcoded dev secrets for `JWT_SECRET` and `SIGNING_KEY_KEK` — see `[F-001]`.
- `[N/A]` `RS-018..021`: Solana Transaction v1 / Geyser RPC — not a Solana indexer/keeper.

#### Checklist 08 — TypeScript Safety (64 items)
- `[PASS]` `TS-001..010`: Zero `: any`, `as any`, `<any>`, `Record<string, any>`, `Promise<any>` across all 63 `.ts` files (`apps/web`, `packages/*`, `examples/*`).
- `[PASS]` `TS-011..019`: No empty `catch {}` blocks; clean error classes in `packages/core/src/errors.ts`; zero `console.log` calls in production frontend/SDK code.
- `[PASS]` `TS-022..030`: ES modules only, strict `tsconfig.json`, TypeScript `~6.0.3`, zero `@ts-ignore`/`@ts-nocheck`/`@ts-expect-error`, zero `eval()`/`new Function()`.
- `[PARTIAL]` `TS-041..048`: Frontend/SDK validates inputs via explicit TypeScript guards (`apps/web/src/lib/control/validation.ts`) rather than Zod schemas; backend Rust validates via `serde` + explicit domain validators.
- `[N/A]` `TS-021, TS-024, TS-031..040, TS-051, TS-060..064`: Solana/Anchor/MongoDB-specific items — not applicable.

#### Checklist 09 — Backend Security (132 items)
- `[PASS]` `BE-015..020, BE-036..044, BE-045..056, BE-061..064, BE-077..081, BE-091..098, BE-104..117, BE-124`: Parameterized SQLx queries, `AppError` JSON error envelopes with zero stack traces, strict security headers on `auth-api`, explicit dynamic CORS allowlists (no `*`), 64 KiB body limit, constant-time HMAC + SHA-256 secret-key lookups, per-route scope enforcement (`SecretKeyAuth::require_scope`), immediate key revocation & cache invalidation, `aud`/`iss`/`typ` JWT pinning, PostgreSQL RLS policies.
- `[FAIL-8]` `BE-066`: Hardcoded fallback secrets in `config/mod.rs:70,84` — see `[F-001]`.
- `[FAIL-6]` `BE-034`, `BE-035`: `PeerIpKeyExtractor` in-memory rate limiter collapses behind NGINX Ingress and is not shared across K8s replicas — see `[F-003]`.
- `[FAIL-6]` `BE-102`: No TOTP/WebAuthn step-up MFA for sensitive control-plane credential/webhook mutations — see `[F-004]`.
- `[FAIL-5]` `BE-099`: Registration & email linking leak account existence (`client_auth.rs:252`, `identity_advanced.rs:171`) — see `[F-007]`.
- `[PARTIAL]` `BE-059, BE-125, BE-126`: Runtime `auth-api` runs `sqlx::migrate!("./migrations")` on startup (`main.rs:32`) using the same DB role rather than a separate migration job/role.
- `[N/A]` `BE-001..009, BE-012..014, BE-021..028, BE-082..090, BE-092..093, BE-101, BE-103, BE-118..123, BE-127..132`: Solana wallet-sig/on-chain/MongoDB/Supabase/Firebase/GraphQL/WebSocket/DeFi market-data items — not applicable.

#### Checklist 10 — Frontend Security (84 items)
- `[PASS]` `FE-001..016, FE-025..031, FE-057..065, FE-071..076`: Zero `dangerouslySetInnerHTML`, `{@html`, `innerHTML`, `document.write`, `eval()`, `console.log`, `postMessage`, or `target="_blank"` risks; `packages/core/src/client.ts:111` stores only ephemeral PKCE `state`/`nonce`/`codeVerifier` in `sessionStorage` (tokens use `HttpOnly` cookies on SSR/IdP); `apps/web/src/hooks.server.ts` enforces server-side session guards and sets `HttpOnly` + `SameSite=Lax` + `Secure` cookies; `@secure-auth/next` Edge middleware verifies ES256 JWTs and strips spoofed headers (`packages/next/test/`).
- `[FAIL-5]` `FE-066`, `FE-070`: `apps/web/src/hooks.server.ts` omits CSP and `X-Frame-Options` headers on SSR HTML — see `[F-006]`.
- `[PARTIAL]` `FE-027`: Cookies use `SameSite=Lax` (appropriate for OIDC top-level redirects; double-submit CSRF mitigates mutating cross-site POSTs).
- `[N/A]` `FE-032..040, FE-041..047, FE-067..069, FE-077..084`: Solana wallet-adapter, Next.js-app-internal image/RSC checks (our web app is SvelteKit; Next.js SDK is a library + reference starter).

#### Checklist 11 — Supply Chain & Dependencies (62 items)
- `[PASS]` `SC-001..006, SC-011..012, SC-025..030, SC-033..038, SC-043..049, SC-051..052`: Zero `axios@1.14.1/0.30.4` or `@solana/web3.js@1.95.6/7`; `npm audit --workspaces --audit-level=high` = `0` vulnerabilities; `npm run check:vendor` verifies `vendor/picomatch` integrity/patch/license; `Cargo.lock` and `package-lock.json` committed; Docker & CI use `npm ci --ignore-scripts` and `cargo build --locked`; weekly scheduled `security.yml` runs `cargo audit`, `npm audit`, `gitleaks`, and `Trivy` + SPDX SBOM generation.
- `[PARTIAL]` `SC-004`: `cargo audit --deny unsound` runs in CI (`security.yml:36`), not locally (`cargo-audit` not installed on host).
- `[PARTIAL]` `SC-013..016`: Some `package.json` runtime dependencies use caret/tilde ranges (`^`/`~`) rather than exact pins (mitigated by committed `package-lock.json` + `npm ci`).
- `[FAIL-4]` `SC-062`: No `.github/dependabot.yml` / Renovate config enforcing review gates on crypto crate/package updates — see `[F-008]`.
- `[N/A]` `SC-022, SC-031..032, SC-050, SC-053..058`: Anchor/Solana/Turborepo/vendored-program items — not applicable.

#### Checklist 12 — Secrets & Key Management (53 items)
- `[PASS]` `SEC-001..010, SEC-016, SEC-018..021, SEC-023..024, SEC-046..053`: Zero hardcoded private keys, keypairs, or `.env` files in current tree or entire Git history (`git log --all` verified); `.env.example` clean; `.gitignore` covers `.env*`, `*.pem`, `*.key`, `secrets/`, `*.tfstate`; Dockerfiles bake zero secrets; CSPRNG (`OsRng`/`SystemRandom`) used for all keys/tokens; emergency ES256 key rotation & immediate revocation verified against Postgres 16.
- `[FAIL-8]` `SEC-012`, `SEC-013`: `JWT_SECRET` and `SIGNING_KEY_KEK` fail open to hardcoded dev defaults (`config/mod.rs:70,84`) and are missing from K8s base secrets — see `[F-001]`.
- `[PARTIAL]` `SEC-022`: Gitleaks runs in GitHub Actions (`security.yml:55-68`), but no local pre-commit secret hook is enforced.
- `[N/A]` `SEC-025..029, SEC-033..040`: Solana RPC/wallet/MongoDB items — not applicable.

#### Checklist 13 — Deployment & Infrastructure (96 items)
- `[PASS]` `DEP-022..024, DEP-028, DEP-033..034, DEP-068..071, DEP-075, DEP-090..096`: Multi-stage Alpine Dockerfiles (`USER 65534:65534` on `auth-api`, `USER appuser` on `web`, `npm`/`yarn` stripped from web runtime); K8s `Deployment` enforces `runAsNonRoot: true`, `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`, `capabilities.drop: ["ALL"]`, `RollingUpdate`, `podAntiAffinity`, `readinessProbe`, `livenessProbe`; `release.yml` `workflow_run` properly verifies `event == 'push'`, `conclusion == 'success'`, `head_repository.full_name == github.repository`, validates tag regex, and restores no untrusted GHA cache (`KV-140` PASS!).
- `[FAIL-5]` `DEP-049`: Live database backup/PITR restoration drill documented (`docs/disaster-recovery.md`) but not yet executed against staging — see `[F-012]`.
- `[FAIL-4]` `DEP-067`: GitHub Actions use mutable tags (`@v7`, `@v2`) instead of commit SHAs — see `[F-008]`.
- `[FAIL-5]` `DEP-076`: Hardcoded fallback dev secrets & missing `SIGNING_KEY_KEK` in K8s `secrets.yaml` — see `[F-001]`.
- `[PARTIAL]` `DEP-080..086`: External Secrets Operator manifests exist (`infrastructure/kubernetes/base/secrets/external-secrets/`), and `release.yml` updates `sha-<commit>` tags rather than immutable `@sha256:` image digests; full Terraform/cloud IaC rollout is Phase 11.
- `[N/A]` `DEP-001..021, DEP-042..044, DEP-061..062, DEP-065, DEP-078..079, DEP-087..089`: Solana/Anchor program deploy & v1 feature gates — not applicable.

#### Checklist 16 — Formal Verification & Testing (73 items)
- `[PASS]` `FV-007, FV-013..019, FV-035..040, FV-045..048, FV-050, FV-052..053, FV-056..057`: `cargo clippy -D warnings`, `cargo fmt --check`, `eslint .`, `svelte-check`, `tsc --noEmit`, `cargo audit`, `npm audit`, Trivy, and Gitleaks all run in CI; 62 Rust unit/integration + 24 live-Postgres integration + 70 JS/TS/Web tests cover positive, negative, edge, and cross-tenant isolation paths; circuit breaker & transactional rollback verified.
- `[FAIL-6]` `FV-049`: No `CatchPanicLayer` + `panic = "abort"` — see `[F-002]`.
- `[FAIL-5]` `FV-054`: Unbounded SMTP `tokio::spawn` discards delivery errors with `let _ =` — see `[F-011]`.
- `[FAIL-6]` `FV-023`, `FV-030`: Zero fuzzing (`cargo-fuzz`) or property-based tests (`proptest`) on JWT, redirect URI, OIDC, or webhook parsers — see `[F-020]`.
- `[FAIL-4]` `FV-033`, `FV-034`: No automated code coverage measurement or threshold reporting in CI — see `[F-020]`.
- `[PARTIAL]` `FV-044`: k6 load/soak suite exists (`tests/load/`), pending staging baseline run — see `[F-015]`.
- `[N/A]` `FV-006, FV-010, FV-024, FV-055, FV-059..073`: Solana on-chain SVM/LiteSVM/Mollusk/Trident items — not applicable.

#### Checklist 17 — Logging, Monitoring & Incident Response (65 items)
- `[PASS]` `LM-007..012, LM-015..019, LM-025..027, LM-031, LM-034, LM-039, LM-041, LM-052..053, LM-055, LM-059, LM-061`: Structured JSON logging (`tracing-subscriber`), W3C `traceparent` + `x-request-id` correlation, automated PII/secret redaction (`phase09_observability_tests.rs`), Prometheus `/metrics` with cardinality guardrails, 10 alert rules (`alerts.yml`), 4 Grafana dashboards, `docs/observability-runbook.md`, `docs/disaster-recovery.md`, automated emergency key-revocation drill.
- `[FAIL-5]` `LM-054`: Live PITR backup restore not yet executed on staging — see `[F-012]`.
- `[FAIL-4]` `LM-051`: No `SECURITY.md` responsible disclosure policy at repository root — see `[F-020]`.
- `[PARTIAL]` `LM-020..022, LM-032, LM-035, LM-042..050`: Production log aggregation/90d immutable retention, Alertmanager PagerDuty/Slack escalation, and tabletop IR drill remain to be wired in Phase 11 cloud deployment — see `[F-019]`.
- `[N/A]` `LM-001..005, LM-014, LM-028..029, LM-037..038, LM-043..044, LM-058, LM-060, LM-063..065`: On-chain Solana event/vault/mint/Geyser items — not applicable.

#### Checklist 18 — Privacy, Compliance & Change Management (60 items)
- `[PASS]` `PC-003, PC-007, PC-010, PC-014, PC-017, PC-027, PC-031, PC-035, PC-038, PC-048, PC-050`: GDPR data export endpoint, plaintext-secret-free logs, OWASP ASVS L2/3 mapping (`docs/security-assessment.md`), Git-managed changes, CI test gating, versioned forward-only SQL migrations (`001..014`), dev/prod environment separation, automated API security test suite.
- `[FAIL-5]` `PC-008`, `PC-009`: `delete_user_account` soft-deletes without anonymizing PII and no retention purge job exists — see `[F-016]`.
- `[FAIL-4]` `PC-001`, `PC-005`, `PC-006`, `PC-015`, `PC-024`: Missing PII inventory, Privacy Policy, ToS, regulatory mapping, and documented PII encryption at rest — see `[F-017]`, `[F-018]`.
- `[FAIL-3]` `PC-029`, `PC-036`: No `.github/CODEOWNERS` or `CHANGELOG.md` — see `[F-020]`.
- `[N/A]` `PC-012, PC-022..023, PC-034, PC-043, PC-051..060`: Crypto wallet/DeFi/AML/AI-ML inference items — not applicable.

---

## 6. Known Attack Vector Results (KV-001..KV-140)

### Out-of-Scope / Feature-Absent Gates
- **KV-001..KV-030, KV-053, KV-065, KV-067, KV-091, KV-101..KV-111, KV-113..KV-123, KV-125, KV-127..KV-138 (65 vectors):** `[N/A — out of scope: off-chain Identity & Sync platform; no Solana/Anchor on-chain program, DeFi pool, wallet adapter, or AI agent runtime]`.
- **KV-031 (MongoDB), KV-034 (Supabase/Firebase), KV-042 (XXE), KV-046 (GraphQL), KV-047/099 (WebSockets), KV-063 (postMessage), KV-066 (user CSS), KV-068/069 (3rd-party CDN scripts):** `[N/A — feature absent: verified 0 grep hits across repository]`.

### Evaluated In-Scope Known Vectors

| Vector | Title | Verdict | Evidence / Reference |
| --- | --- | --- | --- |
| `KV-032` | SQL Injection | `[PASS]` | All SQLx queries use `$1..$N` bind parameters across `apps/auth-api/src/`; 0 raw interpolation |
| `KV-033` | Mass Assignment | `[PASS]` | Typed Rust `Deserialize` DTOs explicitly pick updatable columns |
| `KV-035` | JWT Algorithm Confusion | `[PASS]` | `tokens.rs:278,361,423` pin `Algorithm::ES256` & `typ`; tested in `security_owasp_asvs_tests.rs` |
| `KV-036` / `085` | SSRF | `[PASS]` | `webhooks.rs:257-370` URL validator + `SsrfSafeDnsResolver` + `Policy::none()` + tests |
| `KV-037` / `088` | CORS Misconfiguration | `[PASS]` | `dynamic_cors.rs:65-119` explicit allowlists, no `*`, `Vary: Origin` |
| `KV-038` | IDOR | `[PASS]` | Composite `(environment_id, id)` filters + `security_red_team_isolation_tests.rs` |
| `KV-039` / `083` | Rate Limiting Bypass / Missing | `[FAIL-6]` | `rate_limit.rs:8-48` in-memory `PeerIpKeyExtractor` breaks behind NGINX Ingress & multi-pod (`F-003`) |
| `KV-040` | Command Injection | `[PASS]` | Zero `std::process::Command` / `child_process` calls in app runtime |
| `KV-041` | Path Traversal / LFI | `[PASS]` | No user-controlled filesystem paths; `domains.rs:182` blocks `/../` in redirect URIs |
| `KV-043` / `084` | Prototype Pollution | `[PASS]` | No unsafe deep-merge; `packages/core` uses `Object.create`/explicit property picks |
| `KV-044` | Server-Side Template Injection | `[PASS]` | Svelte auto-escapes templates; zero `{@html` sinks; email HTML escapes values |
| `KV-045` | Webhook Forgery | `[PASS]` | HMAC-SHA256 + 300s timestamp window + `ConstantTimeEq` (`webhooks.rs:154-196`) |
| `KV-048` | ReDoS | `[PASS]` | Vendored & patched `vendor/picomatch` (4 regression tests); Rust `regex` guarantees linear time |
| `KV-049` | HTTP Response Splitting | `[PASS]` | `axum`/`http::HeaderValue::from_str` rejects CR/LF in header values |
| `KV-050` | Session Fixation | `[PASS]` | Fresh CSPRNG session & refresh tokens minted on every login/exchange |
| `KV-051` | Account Enumeration | `[FAIL-5]` | Sign-in has timing parity, but `initiate_sign_up` & `add_user_email` disclose existing emails (`F-007`) |
| `KV-052` | Unbounded Request Body DoS | `[PASS]` | `RequestBodyLimitLayer::new(64 * 1024)` (`routes/mod.rs:77`) + Ingress `64k` |
| `KV-054` | Default Credentials in Production | `[FAIL-8]` | `config/mod.rs:70,84` hardcoded fallback `JWT_SECRET` & `SIGNING_KEY_KEK` + K8s secret omission (`F-001`) |
| `KV-055` / `082` | Exposed Debug / Admin Endpoints | `[FAIL-4]` | `/metrics` exposed unauthenticated on main port `8080` (`routes/mod.rs:58-68`, `F-008`) |
| `KV-056..058` | XSS (SVG / Stored / DOM) | `[PASS]` | Zero `dangerouslySetInnerHTML`, `{@html`, `innerHTML`, `document.write`, `eval` |
| `KV-059` | Clickjacking | `[FAIL-5]` | `auth-api` sets `X-Frame-Options: DENY`, but `apps/web/src/hooks.server.ts` omits it (`F-006`) |
| `KV-060` | OAuth State Forgery (CSRF) | `[PASS]` | CSPRNG `state` + `nonce` + PKCE `S256` validated server-side (`social_oauth.rs`, `oidc_provider.rs`) |
| `KV-061` | Sensitive Data in URL Params | `[PASS]` | Auth tokens passed in POST JSON / `HttpOnly` cookies; OIDC uses short-lived single-use `code` |
| `KV-062` | Client-Side Auth Bypass | `[PASS]` | All API endpoints enforce server-side extractors (`AuthenticatedClientUser`, `AuthenticatedDeveloper`, `SecretKeyAuth`) |
| `KV-064` | LocalStorage Token Theft | `[PASS]` | Zero `localStorage` usage; `sessionStorage` in `packages/core/src/client.ts:111` stores only ephemeral PKCE verifier |
| `KV-070` | Open Redirect | `[PASS]` | Exact registered `redirect_uri` validation + parser-differential tests (`security_owasp_asvs_tests.rs`) |
| `KV-071` | Missing CSP | `[FAIL-5]` | Missing CSP in `apps/web/src/hooks.server.ts` (`F-006`) |
| `KV-072` | API Key Exposure in Client Bundle | `[PASS]` | Only `pk_*` publishable keys reach client; `sk_*` keys restricted to server helpers |
| `KV-073` / `092` | Dangling DNS / Domain Takeover | `[PARTIAL]` | Pre-production — verify DNSSEC/CAA/CNAME hygiene during Phase 11 cloud DNS provisioning |
| `KV-074` | Insecure External Link | `[PASS]` | Zero `target="_blank"` occurrences without `rel` |
| `KV-075` | Console Data Leak | `[PASS]` | Zero `console.log`/`debug`/`warn`/`error` calls in `apps/web/src/` and `packages/` |
| `KV-076` | Dependency Confusion | `[PASS]` | Private workspace packages (`@secure-auth/*`), locked `package-lock.json` |
| `KV-077` | Malicious npm Package | `[PASS]` | Zero compromised `axios`/`@solana/web3.js`; `npm audit` = 0; `check:vendor` PASS |
| `KV-078` | Secrets in Git History | `[PASS]` | `git log --all` verified 0 committed `.env`/`.pem`/keypair files |
| `KV-079` | `.env` Committed to Repo | `[PASS]` | Only `.env.example` templates tracked; `.env*` in `.gitignore` |
| `KV-080` | CI/CD Pipeline Injection | `[PARTIAL]` | No untrusted `${{ github.event.* }}` interpolation in `run:` blocks; mutable action tags remain (`F-008`) |
| `KV-081` | Insecure Docker Configuration | `[PASS]` | Multi-stage Alpine, non-root users (`65534` / `1001`), `npm`/`yarn` stripped from web runtime |
| `KV-086` | Insecure Deserialization | `[PASS]` | Strict `serde_json` typed deserialization; no `pickle`/`yaml.load` |
| `KV-087` | Insufficient Logging & Monitoring | `[PARTIAL]` | Rich OTel/Prometheus/Grafana/alerts exist; production Alertmanager & 90d log sink pending Phase 11 (`F-019`) |
| `KV-089` | Unpatched Server Dependencies | `[PASS]` | `npm audit` = 0; `cargo audit --deny unsound` + Trivy HIGH/CRITICAL gate in `security.yml` |
| `KV-090` | Missing HTTPS / TLS | `[PARTIAL]` | `ingress.yaml` enforces HTTPS + Let's Encrypt; `secrets.yaml` lacks `COOKIE_SECURE=true` (`F-001`) |
| `KV-093` | Improper Error Handling | `[PASS]` | `AppError` maps internal DB/Redis/crypto errors to generic JSON envelopes (`errors/mod.rs:130-174`) |
| `KV-094` | Missing Input Length Limits | `[FAIL-4]` | Body capped at 64 KiB, but password length has no 128-byte cap prior to Argon2id (`F-010`) |
| `KV-095` | Insecure Randomness | `[PASS]` | `OsRng` / `SystemRandom` / `crypto.getRandomValues` used everywhere; zero `Math.random()` |
| `KV-096` | Missing Security Headers | `[PARTIAL]` | `auth-api` & K8s Ingress PASS; `apps/web` SSR missing headers (`F-006`) |
| `KV-097` | Stale / Leaked Dev Credentials | `[FAIL-8]` | Hardcoded dev `JWT_SECRET` & `SIGNING_KEY_KEK` fallbacks active if env missing (`F-001`) |
| `KV-098` | Broken Access Control | `[PASS]` | RBAC & tenant isolation verified across all 3 planes (`phase07`, `phase08`, `security_red_team_isolation_tests.rs`) |
| `KV-100` | Insufficient Backup / DR | `[FAIL-5]` | Runbook documented, but live staging PITR restore drill not yet executed (`F-012`) |
| `KV-112` | In-Memory Secret Non-Zeroization | `[FAIL-5]` | ES256 private keys & webhook secrets not wrapped in `Zeroizing` (`F-005`) |
| `KV-124` | Custodial Cleartext Key Export | `[PASS]` | Private signing keys & webhook secrets encrypted at rest with AES-256-GCM + AAD; secret keys hashed with SHA-256 |
| `KV-126` | Session Token as Custody | `[FAIL-6]` | Privileged control-plane key/webhook mutations lack step-up MFA (`F-004`) |
| `KV-139` | Comment-Triggered GHA Without Author Gate | `[N/A — feature absent]` | No `issue_comment`, `issues`, `discussion*`, or AI-agent workflows in `.github/workflows/` |
| `KV-140` | `workflow_run` / `pull_request_target` Pwn Request | `[PASS]` | `release.yml` `workflow_run` gates on `event == 'push'` + `head_repository.full_name == github.repository`, checks out base `main`, validates tag regex, uses no untrusted artifacts or caches |

---

## 7. Code Maturity Scorecard (Phase 4.5 — Scale 0–4)

| # | Category | Score (0–4) | Justification |
| --- | --- | :---: | --- |
| 1 | Access Controls | **3 / 4 (Good)** | Strong multi-tenant RBAC, composite FKs, RLS, scoped `sk_*` keys; deduct 1 for missing step-up MFA on control plane (`F-004`) |
| 2 | Arithmetic & Bounds | **3 / 4 (Good)** | `overflow-checks = true` in release profile, clamped pagination/concurrency; deduct 1 for missing password max-length bound (`F-010`) |
| 3 | Type & Memory Safety | **3 / 4 (Good)** | Strong Rust newtype IDs, zero `unsafe`, zero TypeScript `any`; deduct 1 for non-zeroized secret buffers (`F-005`) |
| 4 | Input Validation | **3 / 4 (Good)** | Strict redirect URI, origin, slug, SSRF URL/DNS validators; deduct 1 for sign-up enumeration & secondary OTP handling (`F-007`, `F-009`) |
| 5 | Testing | **3 / 4 (Good)** | 132 passing tests + 24 live-Postgres integration tests including red-team & ASVS suites; deduct 1 for missing CI coverage gate (`F-020`) |
| 6 | **Fuzzing & Property Tests** | **0 / 4 (Absent)** | **Priority Remediation:** Zero `cargo-fuzz` or `proptest` targets (`FV-023`, `FV-030`, `F-020`) |
| 7 | Error Handling & DoS Resilience | **2 / 4 (Partial)** | Clean error envelopes & bounded Argon2id pool, but `panic = "abort"` without `CatchPanicLayer`, proxy-broken rate limiter, and unbounded SMTP `tokio::spawn` (`F-002`, `F-003`, `F-011`) |
| 8 | Deployment & Secret Operations | **2 / 4 (Partial)** | Hardened Dockerfiles & K8s pod security context, but fail-open dev fallback secrets, incomplete K8s `secrets.yaml` & `ingress.yaml`, and floating GHA tags (`F-001`, `F-008`, `F-012`) |
| 9 | Monitoring & Incident Response | **3 / 4 (Good)** | End-to-end OTel, Prometheus metrics/alerts, 4 Grafana dashboards, observability & DR runbooks; deduct 1 for unexecuted live PITR drill & unseparated `/metrics` port (`F-012`, `F-019`) |

---

## 8. Prioritized Remediation Roadmap

### P0 — Deploy Blockers (Fix Before Any Production Rollout)
1. **`F-001` (Sev 8 — HIGH):** Remove hardcoded fallback values for `JWT_SECRET` and `SIGNING_KEY_KEK` in `apps/auth-api/src/config/mod.rs` (fail closed at boot), and add `SIGNING_KEY_KEK`, `SIGNING_KEY_KEK_VERSION`, `COOKIE_SECURE=true`, and `ISSUER_URL` to `infrastructure/kubernetes/base/auth/secrets.yaml` and `externalsecret-auth.yaml`.
2. **`F-003` (Sev 6 — MEDIUM):** Replace in-memory `PeerIpKeyExtractor` in `apps/auth-api/src/middleware/rate_limit.rs` with a Redis-backed distributed rate limiter supporting trusted proxy hops + per-environment/per-key quotas.
3. **`F-002` (Sev 6 — MEDIUM):** Switch `[profile.release]` to `panic = "unwind"`, add `tower_http::catch_panic::CatchPanicLayer` in `apps/auth-api/src/routes/mod.rs`, and replace `.expect(...)` in `oidc_provider.rs:780,795`.
4. **`F-012` (Sev 5 — MEDIUM):** Add `/v1`, `/oauth2`, `/.well-known`, and `/api` routes to `infrastructure/kubernetes/base/auth/ingress.yaml`, and execute + sign off live staging PITR restore & chaos drills.

### P1 — Pre-Launch Security, Privacy & Maturity Upgrades (Within 2 Weeks)
5. **`F-004` (Sev 6 — MEDIUM):** Implement TOTP (`RFC 6238`) + hashed recovery codes (and WebAuthn/Passkeys) to back `require_mfa` and developer control-plane step-up auth.
6. **`F-005` (Sev 5 — MEDIUM):** Implement redacting `fmt::Debug` on `AppConfig` and wrap decrypted ES256 private keys & webhook secrets in `zeroize::Zeroizing`.
7. **`F-006` (Sev 5 — MEDIUM):** Configure SvelteKit CSP and attach security headers (`X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy`, `HSTS`) in `apps/web/src/hooks.server.ts`.
8. **`F-016` (Sev 5 — MEDIUM):** Anonymize user PII columns on `delete_user_account` and add a scheduled retention-purge worker for expired audit/outbox records.
9. **`F-007` & `F-009` (Sev 5 — MEDIUM):** Eliminate sign-up/email enumeration, hash pending sign-up OTPs at rest, use `ConstantTimeEq` for secondary email OTPs, and enforce a 5-attempt cap.
10. **`F-011` & `F-013` (Sev 5 — MEDIUM):** Replace fire-and-forget SMTP `tokio::spawn` with a bounded worker/outbox queue with error metrics, and reuse shared `OnceLock<reqwest::Client>` instances in `webhooks.rs` and `social_oauth.rs`.
11. **Maturity Category 6 (0/4 — Priority):** Add `proptest` / `cargo-fuzz` targets for `domains::validate_redirect_uri`, `webhooks::validate_webhook_url`, `webhooks::verify_signature_header`, and JWT parsing.

### P2 — Next Sprint (Low-Severity Hardening)
12. **`F-008` (Sev 4):** Pin all GitHub Actions to full 40-char commit SHAs + add `.github/dependabot.yml`; move `auth-api` `/metrics` to dedicated internal port `9090`.
13. **`F-010` (Sev 4):** Enforce `password.len() <= 128` before Argon2id hashing/verification.
14. **`F-014` (Sev 4):** Add a short-TTL in-memory `kid → DecodingKey` cache invalidated on key rotation/revocation.
15. **`F-017` & `F-018` (Sev 4):** Publish PII data inventory, Privacy Policy, Terms of Service, and SOC 2 / GDPR control mapping.

### P3 — Backlog & Launch Polish
16. **`F-015`, `F-019`, `F-020` (Sev 3):** Execute k6 staging load/soak baselines, wire Alertmanager receivers & 90-day immutable log sink (Phase 11), and add `.github/CODEOWNERS`, `CHANGELOG.md`, root `LICENSE`, `SECURITY.md`, and CI coverage tracking.

---

## 9. Notes & Nitpicks (Below Severity 1 — No Security Impact)

- `apps/web/svelte.config.js` / `apps/web/tsconfig.json`: `svelte-check` emits a deprecation notice for `config.alias` (`config_option_deprecated_alias`); migrate to package subpath imports before SvelteKit 4.
- `apps/auth-api/Dockerfile` and `docker/Dockerfile.api` (and `apps/web/Dockerfile` vs `docker/Dockerfile.web`) maintain near-duplicate Dockerfile pairs; consolidate to a single canonical pair to avoid future drift.
- `apps/auth-api/Cargo.toml:7`: `authors = ["Your Name <you@example.com>"]` still uses a placeholder author string.

---

## 10. Appendices & Audit Disclaimer

- **Toolchain Verified:** Rust `1.96.0` (`cargo fmt`, `cargo clippy`, `cargo test`), Node.js `v24.11.1` (`npm`, `tsc ~6.0.3`, `eslint 10.9.1`, `svelte-check 4.0.0`, `vitest 5.1.1`).
- **Disclaimer:** This audit was performed using `auditor-skill` v7.6 automated and manual source-code inspection at commit `379fa6a`. It provides an evidence-backed engineering security baseline and does not substitute for an independent external penetration test, formal cryptographic review, or legal compliance certification prior to regulated production launch.
