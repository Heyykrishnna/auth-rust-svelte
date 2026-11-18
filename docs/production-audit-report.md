# Comprehensive Security & Production Readiness Audit Report (Re-Audit v2)

> **Framework Applied:** `auditor-skill` v7.6 (Scope-Gated Multi-Language Full Audit + `/auditor:re-audit` Fix Verification & Sibling-Patch-Propagation Sweep — CertiK / EY SOC 2 / COBIT / OWASP Top 10:2025 / OWASP ASVS L2-L3)  
> **Target Repository:** `auth-rust-svelte` (Dradix Multi-Tenant Identity & Sync Platform)

---

## 1. Executive Summary

| Field | Value |
| --- | --- |
| **Repository** | `Heyykrishnna/auth-rust-svelte` |
| **Branch / Audited Commit** | `main` @ `61cdbcb` *(Delta from Baseline `379fa6a`: `379fa6a..61cdbcb`, 8 commits)* |
| **Audit Date** | `2026-11-18` |
| **Audit Scope** | `FULL` + `RE-AUDIT` (10 In-Scope Checklists / 710 Items + 140 Known Vectors + Sibling Sweep) |
| **Security Risk Score** | **6 — MEDIUM** *(down from **8 — HIGH** at `379fa6a`; 0 Critical, 0 High security vulns)* |
| **Build / Release Gate** | **BLOCKED — Active Rust Compile Regression (`F-023`, `tokens.rs:521,550`)** |
| **Overall Production Score** | **87.0 / 100** *(**+13.0 pts** vs `74.0 / 100` baseline; **89.0 / 100** once 2-char `F-023` borrow typo is fixed)* |

### What We Found

Between baseline commit `379fa6a` and `HEAD` (`61cdbcb`), the engineering team landed substantial security, privacy, observability, and reliability hardening across `c0ca048`, `132b24a`, `074a70b`, `e0195b0`, and `61cdbcb`. **Every High-severity security deploy blocker from the prior audit has been eliminated:**

- **Secrets & Boot Safety (`F-001` FIXED):** `AppConfig::from_env()` now fails closed on missing, short, or placeholder `JWT_SECRET`/`SIGNING_KEY_KEK` values and enforces `COOKIE_SECURE=true` + HTTPS `ISSUER_URL` when `APP_ENV=production`. Kubernetes `secrets.yaml` and `externalsecret-auth.yaml` now wire all required keys, and `scripts/setup-env.mjs` generates `0o600` local `.env` files safely.
- **Memory & Panic Hygiene (`F-002`, `F-005` FIXED):** `Cargo.toml` switched to `panic = "unwind"` with `CatchPanicLayer` in `routes/mod.rs`, zero `.unwrap()`/`.expect()` remain in request handlers, `AppConfig` redacts secrets in `fmt::Debug` and zeroizes on `Drop`, and `signing_keys`, `password_hasher`, `webhooks`, and `social_oauth` use `zeroize::Zeroizing`.
- **Distributed Rate Limiting, OTP, Passwords & Privacy (`F-003` proxy, `F-006`, `F-009`, `F-010`, `F-011`, `F-013`, `F-016` FIXED):** Added atomic Redis Lua GCRA rate limiting with trusted-proxy CIDR `X-Forwarded-For` walking, atomic Redis Lua OTP verification (`services/otp.rs`, SHA-256 + CT compare + 5-attempt cap), 128-char/512-byte password limits, SvelteKit 3 CSP + SSR security headers, shared `OnceLock<reqwest::Client>` pools, dedicated internal `/metrics` listener on port `9090`, SHA-pinned GitHub Actions + `.github/dependabot.yml`, full GDPR erasure anonymization in `delete_user_account`, and scheduled hourly data-retention purging (`services/retention.rs`, migration `015`).

**Three newly introduced issues + remaining medium gaps prevent immediate production sign-off:**
1. **`[F-023]` Active Compile Break (`tokens.rs:521,550`):** Commit `61cdbcb` calls `cached_decoding_key(jwk)?` instead of `cached_decoding_key(&jwk)?`, failing `cargo test` / `rust-ci` (`E0308`).
2. **`[F-021]` Redis Outage Fail-Closed (`rate_limit.rs:139-157`):** The new Redis rate limiter returns `503 Service Unavailable` for **all** `/v1/*`, `/oauth2/*`, and `/auth/*` traffic on any Redis error, conflicting with the graceful-degradation DR policy (`docs/disaster-recovery.md` §1).
3. **`[F-004]` Missing MFA Implementation & `[F-007]` Unpatched Sibling:** TOTP/WebAuthn MFA is still unimplemented behind `require_mfa`, and legacy `authentication.rs:44-46` (`POST /auth/register`) was missed during the `client_auth.rs` sign-up enumeration fix.

---

### 7-Pillar Production Readiness Scorecard (100 Points)

| Aspect | Baseline (`379fa6a`) | Current (`61cdbcb`) | Max | Delta | Key Drivers & Remaining Deductions |
| --- | ---: | ---: | ---: | ---: | --- |
| **Security** | 22.0 | **28.5** | 30 | **+6.5** | All prior High/Medium security flaws closed (`F-001`, `F-003` proxy spoofing, `F-005`, `F-006`, `/metrics` port, SHA-pinned GHA + Dependabot); **-0.5** legacy registration enumeration sibling (`F-007`), **-0.5** `verify_user_email` unique-index 500 + unused `tower_governor` deps (`F-022`), **-0.5** missing `# vX.Y.Z` tag comments on SHA pins (`F-008` residual) |
| **Authentication** | 15.0 | **17.0** | 20 | **+2.0** | Atomic Redis Lua OTP + SHA-256 pending OTPs (`F-009`) & 128-char password cap (`F-010`) closed; **-2.0** no TOTP/WebAuthn MFA flow backing `require_mfa` (`F-004`), **-1.0** `authentication.rs` sibling enumeration + SMTP timing differential (`F-007`) |
| **Reliability** | 11.0 | **11.5** | 15 | **+0.5** | `CatchPanicLayer` + `unwind` (`F-002`), bounded SMTP timeouts (`F-011`), and K8s Ingress routes closed; **-1.5** active Rust compile break at `tokens.rs:521,550` (`F-023`), **-1.5** Redis outage fail-closed `503` across all routes (`F-021`), **-0.5** live staging PITR/chaos drills pending Phase 11 (`F-012`) |
| **Performance** | 11.0 | **13.0** | 15 | **+2.0** | Shared `OnceLock<reqwest::Client>` pools (`F-013`) & EC `DecodingKey` cache (`tokens.rs:689`) landed; **-1.0** DB query to `signing_keys` still runs per JWT verify (`F-014` partial), **-1.0** live staging k6/soak baseline pending (`F-015`) |
| **Privacy** | 6.0 | **8.0** | 10 | **+2.0** | Full GDPR PII erasure (`identity_advanced.rs:649-746`) & scheduled hourly retention purge (`retention.rs`, `015_retention_indexes.sql`) closed (`F-016`); **-1.0** no field-level PII encryption or formal PII inventory (`F-017`), **-1.0** missing Privacy Policy/ToS/compliance mapping (`F-018`) |
| **Observability** | 4.5 | **4.75** | 5 | **+0.25** | Dedicated internal `/metrics` port `9090` + email & retention counters added; **-0.25** production Alertmanager escalation & 90d external immutable log sink pending Phase 11 (`F-019`) |
| **Developer Experience** | 4.5 | **4.25** | 5 | **-0.25** | Added `scripts/setup-env.mjs` (`0o600`), `.github/dependabot.yml`, and `security_hardening_tests.rs`; **-0.5** active `HEAD` compile error (`F-023`), **-0.5** missing `CODEOWNERS`, `CHANGELOG.md`, `LICENSE`, `SECURITY.md`, CI coverage & fuzzing (`F-020`) |
| **TOTAL** | **74.0** | **87.0** | **100** | **+13.0** | **Security: STRONG (`28.5/30`) · Overall: B+ (`87.0/100`, `89.0/100` post `F-023` fix)** |

---

### Severity Distribution (Open Findings at `61cdbcb`)

| Score | Label | Open Count | Finding IDs |
| --- | --- | ---: | --- |
| 10 | CRITICAL | 0 | — |
| 9 | CRITICAL | 0 | — |
| 8 | HIGH | 0 | *(Prior `F-001` FIXED)* |
| 7 | HIGH | 0 | — |
| 6 | MEDIUM | 2 | `F-004`, `F-023` *(build/CI blocker)* |
| 5 | MEDIUM | 3 | `F-007` *(partial)*, `F-012` *(partial)*, `F-021` *(new)* |
| 4 | LOW | 5 | `F-008` *(partial)*, `F-014` *(partial)*, `F-017`, `F-018`, `F-022` *(new)* |
| 3 | LOW | 3 | `F-015`, `F-019` *(partial)*, `F-020` |
| 2 | INFO | 0 | — |
| 1 | INFO | 0 | — |
| **Total Open** | | **13** | **0 Critical · 0 High · 5 Medium · 8 Low** *(10 prior findings fully FIXED)* |

---

## 2. Re-Audit Finding-Diff & Sibling-Patch-Propagation Sweep (`379fa6a..61cdbcb`)

### 2.1 Finding-Diff Matrix (`templates/re-audit.md` §1)

| Finding | Orig Sev | Status at `61cdbcb` | Remediation Commit(s) | Evidence (`file:line`) | Verification |
| --- | :---: | --- | --- | --- | --- |
| `F-001` | 8 | **FIXED** | `c0ca048`, `132b24a`, `e0195b0` | `config/mod.rs:69,82,140-142,248-269`, `secrets.yaml:17-21`, `externalsecret-auth.yaml:61-84`, `security_hardening_tests.rs:20-94` | Re-derived from code + automated subprocess boot tests added *(flagged `⚑ no executed revert` on `61cdbcb` solely because `F-023` blocked `cargo test`)* |
| `F-002` | 6 | **FIXED** | `c0ca048` | `Cargo.toml:29,101`, `routes/mod.rs:67`, `oidc_provider.rs:780-797` | Re-derived from code; confirmed 0 `.unwrap()`/`.expect()` in request handlers |
| `F-003` | 6 | **PARTIALLY-FIXED** | `c0ca048`, `074a70b` | `middleware/rate_limit.rs:20-158`, `security_hardening_tests.rs:113-203` | Proxy spoofing & multi-replica sharing FIXED via Redis Lua GCRA + trusted CIDRs; introduced fail-closed Redis outage regression → tracked as `[F-021]` |
| `F-004` | 6 | **STILL-OPEN** | — | `oidc_provider.rs:389`, `control_plane.rs:79` | No TOTP/WebAuthn enrollment or control-plane step-up auth |
| `F-005` | 5 | **FIXED** | `c0ca048`, `074a70b`, `e0195b0` | `config/mod.rs:271-294`, `signing_keys.rs:42,61,89,96,106,115,122`, `password_hasher.rs:59,83`, `webhooks.rs:412,441` | Re-derived from code + unit test `config_debug_omits_credentials_and_private_keys` |
| `F-006` | 5 | **FIXED** | `c0ca048` | `apps/web/vite.config.ts:7-21`, `apps/web/src/hooks.server.ts:84-92` | Re-tested & confirmed via `npm run lint`, `npm run check`, `npm test`, `npm run build` |
| `F-007` | 5 | **PARTIALLY-FIXED** | `c0ca048`, `074a70b`, `e0195b0` | `client_auth.rs:251-261,1608` fixed; `authentication.rs:44-46` STILL returns `EmailAlreadyExists`; `client_auth.rs:305` SMTP timing skew | Sibling sweep caught unpatched legacy registration route |
| `F-008` | 4 | **PARTIALLY-FIXED** | `132b24a`, `61cdbcb` | `/metrics` moved to `127.0.0.1:9090` (`main.rs:46-59`, `routes/mod.rs:81-100`); GHA SHA-pinned (`ci.yml`, `security.yml`, `release.yml`) + `.github/dependabot.yml` | Only `# vX.Y.Z` inline version comments on SHA pins remain missing (metadata tracked in `docs/action-pins.json`) |
| `F-009` | 5 | **FIXED** | `132b24a`, `074a70b` | `services/otp.rs:5-45`, `identity_advanced.rs:171,192`, `client_auth.rs:269,361,574`, `authentication.rs:56,102` | Re-derived from code + concurrent Lua OTP test in `security_hardening_tests.rs:204-224` |
| `F-010` | 4 | **FIXED** | `c0ca048`, `074a70b` | `password_hasher.rs:50,74,98-105`, `security_hardening_tests.rs:136-154` | Re-derived from code (128-char / 512-byte bound before Argon2id) |
| `F-011` | 5 | **FIXED** | `c0ca048`, `074a70b` | `client_auth.rs:305,601`, `authentication.rs:72`, `email.rs:512-529` | All 7 fire-and-forget SMTP `tokio::spawn` calls removed; 10s timeout + `auth_email_delivery_total` metrics added |
| `F-012` | 5 | **PARTIALLY-FIXED** | `132b24a` | `ingress.yaml:72-99` now routes `/v1`, `/oauth2`, `/.well-known`, `/api` | Live staging PITR restore & chaos drill sign-off remains pending Phase 11 |
| `F-013` | 5 | **FIXED** | `c0ca048`, `074a70b` | `webhooks.rs:537,1102-1118`, `social_oauth.rs:382` | Shared `OnceLock<reqwest::Client>` pools verified in code |
| `F-014` | 4 | **PARTIALLY-FIXED** | `61cdbcb` | `tokens.rs:689-713` (`cached_decoding_key`) | EC point parsing cached, but Postgres `find_verifiable_signing_key_by_kid` still runs on every JWT verification (and callers at lines 521, 550 caused `F-023`) |
| `F-015` | 3 | **STILL-OPEN** | — | `tests/load/*.js`, `docs/performance-report.md` | Live staging k6/2h soak baselines pending |
| `F-016` | 5 | **FIXED** | `074a70b` | `identity_advanced.rs:649-746`, `services/retention.rs`, `migrations/015_retention_indexes.sql`, `bin/webhook_worker.rs:93-114` | Full PII anonymization + hourly `SKIP LOCKED` retention purger + test in `security_hardening_tests.rs:228-266` |
| `F-017` | 4 | **STILL-OPEN** | — | Migrations `001..015` | No field-level PII encryption or formal PII Data Inventory doc |
| `F-018` | 4 | **STILL-OPEN** | — | `docs/` | Missing Privacy Policy, Terms of Service, GDPR/SOC 2 compliance matrix |
| `F-019` | 3 | **PARTIALLY-FIXED** | `132b24a` | Dedicated `/metrics` port `9090` + `service-monitor.yaml` + `network-policies.yaml` | Production Alertmanager PagerDuty/Slack receivers & 90d external log sink pending Phase 11 |
| `F-020` | 3 | **STILL-OPEN** | — | Repository root / `.github/` | Missing `.github/CODEOWNERS`, `CHANGELOG.md`, root `LICENSE`, `SECURITY.md`, CI coverage & fuzzing |

### 2.2 Sibling-Patch-Propagation Sweep (`templates/re-audit.md` §3)

| Swept Anti-Pattern Signature | Globs | Un-Patched Siblings Found |
| --- | --- | --- |
| Hardcoded secret fallbacks (`unwrap_or_else.*(secret\|kek\|change_me)`) | `apps/auth-api/src/**/*.rs` | None found |
| Request-reachable `.unwrap()` / `.expect()` | `apps/auth-api/src/**/*.rs` | None found *(only `main.rs:87,93` signal-handler startup)* |
| Unbounded fire-and-forget `tokio::spawn` | `apps/auth-api/src/**/*.rs` | None for SMTP *(remaining 3 are `main.rs:53` metrics server, `webhook_worker.rs:82` metrics server, `middleware/tenant.rs:151` throttled last-used update)* |
| Per-request `reqwest::Client::builder()` | `apps/auth-api/src/**/*.rs` | None found *(only inside `OnceLock` initializers `shared_webhook_client` & `shared_oauth_client`)* |
| Plaintext OTP storage or `==` comparison | `apps/auth-api/src/**/*.rs` | None found *(`otp.rs`, `client_auth.rs`, `authentication.rs` all hash + constant-time compare)* |
| Registration email enumeration (`Err(AppError::EmailAlreadyExists)`) | `apps/auth-api/src/**/*.rs` | **`apps/auth-api/src/services/authentication.rs:45`** (`register_user` on `/auth/register` & `/api/auth/register`) — tracked under `[F-007]` |
| Missing borrow on `cached_decoding_key(&jwk)` | `apps/auth-api/src/services/tokens.rs` | **`tokens.rs:521` & `tokens.rs:550`** — tracked as `[F-023]` |
| Mutable `@v*` tags in GitHub Actions | `.github/workflows/*.yml` | None found *(all 3 workflows pinned to 40-hex SHAs / `@sha256:` digests)* |

---

## 3. Scope Coverage & Methodology (Rules 0, 11a, 11b)

### Scope Coverage Table

| Checklist / Vector Group | Status | Items | Trigger / Reason |
| --- | --- | ---: | --- |
| 01–07 On-Chain Solana (7 checklists) | `OUT-OF-SCOPE` | 0 / 531 | No `Anchor.toml` or `programs/` |
| 08 — TypeScript Safety | `IN-SCOPE` | 64 / 64 | 63 `.ts` files (`apps/web`, `packages/*`, `examples/*`) |
| 09 — Backend Security | `IN-SCOPE` | 132 / 132 | `apps/auth-api` HTTP/OIDC/Backend API + SvelteKit SSR |
| 10 — Frontend Security | `IN-SCOPE` | 84 / 84 | `apps/web` SvelteKit UI + React/Next/Svelte SDKs |
| 11 — Supply Chain & Dependencies | `IN-SCOPE` | 62 / 62 | Universal (`Cargo.lock`, `package-lock.json`, `vendor/picomatch`, Dependabot) |
| 12 — Secrets & Key Management | `IN-SCOPE` | 53 / 53 | Universal (`AppConfig`, `signing_keys`, `setup-env.mjs`, K8s secrets) |
| 13 — Deployment & Infrastructure | `IN-SCOPE` | 96 / 96 | Universal (Dockerfiles, K8s, SHA-pinned GitHub Actions) |
| 14 — Python Safety | `OUT-OF-SCOPE` | 0 / 82 | No Python application files |
| 15 — General Language Safety | `OUT-OF-SCOPE` | 0 / 88 | No Go/Java/Ruby/PHP application files |
| 16 — Formal Verification & Testing | `IN-SCOPE` | 73 / 73 | Universal (Rust tests, Vitest, SDK tests, CI) |
| 17 — Logging, Monitoring & IR | `IN-SCOPE` | 65 / 65 | Universal (OTel, Prometheus `:9090`/`:9091`, Grafana, DR runbooks) |
| 18 — Privacy, Compliance & Change Mgmt | `IN-SCOPE` | 60 / 60 | Universal (GDPR erasure, retention worker, SDLC) |
| 19 — AI Agent Security | `OUT-OF-SCOPE` | 0 / 35 | No `.mcp.json` or LLM/agent runtime |
| 20 — Rust Off-Chain Services | `IN-SCOPE` | 21 / 21 | 109 `.rs` files in `apps/auth-api` |
| Known Vectors (`KV-001..KV-140`) | Scope-Gated | 140 / 140 | 75 in-scope evaluated; 65 on-chain/AI gated `OUT-OF-SCOPE` |

### Live Verification Commands Executed at `e0195b0` / `61cdbcb`

| Command | Commit | Outcome |
| --- | --- | --- |
| `cd apps/auth-api && cargo fmt --check` | `074a70b` / `e0195b0` | **PASS** (0 diffs) |
| `cd apps/auth-api && cargo clippy --locked --all-targets --all-features -- -D warnings` | `e0195b0` | **PASS** (0 warnings) |
| `cd apps/auth-api && cargo test --locked --all-targets --all-features --no-run` | `61cdbcb` | **FAIL (`E0308`)** — 2 missing `&` borrows at `tokens.rs:521,550` (`F-023`) |
| `npm run lint` (`eslint .`) | `e0195b0` / `61cdbcb` | **PASS** (0 errors, 0 warnings) |
| `npm run check` (`shared-types`, 4 SDKs, examples, `svelte-check`) | `e0195b0` / `61cdbcb` | **PASS** (0 errors, 0 warnings) |
| `npm test` (`test:security` + 4 SDKs + `@secure-auth/web` Vitest) | `e0195b0` / `61cdbcb` | **PASS** (**70 / 70 tests**) |
| `npm run build` | `e0195b0` / `61cdbcb` | **PASS** (`@sveltejs/adapter-node` + SvelteKit 3 CSP) |
| `npm run check:vendor` & `npm audit --workspaces --audit-level=high` | `379fa6a` *(lockfiles unchanged for JS)* | **PASS** (0 vulnerabilities) |

- **Execution Mode (Rule 11a):** Native (host). **Outside Services (Rule 11b):** `npm audit` queried `registry.npmjs.org`; `cargo-audit` runs in CI (`security.yml`).

---

## 4. Open & New Findings (Severity ≥ 4 — Rule 5 & Rule 5b)

---

#### [F-023] [NEW] Rust Compilation Regression in `cached_decoding_key` Legacy Callers Breaks CI & Releases

| Field | Value |
| --- | --- |
| **Severity** | **6 — MEDIUM (CI / Release Blocker)** |
| **Checklist Items** | `DEP-060`, `FV-040` |
| **Pillar** | Reliability (-1.5 pts) · Developer Experience (-0.5 pt) |
| **Category** | Build Integrity / Regression |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/tokens.rs:521,550,689` |
| **PoC Evidence Tier** | `[PoC-REPRODUCED]` *(reproduced live via `cargo test --no-run` on `61cdbcb`)* |
| **Status** | Open |

**Description:**
Commit `61cdbcb` defined `fn cached_decoding_key(jwk: &EcPublicJwk)` (`tokens.rs:689`) and updated tenant verifiers with `&jwk`, but passed `jwk` by value at lines `521` (`verify_legacy_es256_access_token`) and `550` (`verify_legacy_es256_refresh_token`), causing `error[E0308]: mismatched types` across `auth-api` lib and test targets.

**Rule 5b Validation Gate:**
- **Reachability:** Every `cargo build`, `cargo clippy`, `cargo test`, and Docker `auth-api` build at `61cdbcb` fails immediately. Verdict: **REACHABLE (REPRODUCED)**.
- **Math / State-Bounds:** `2` type-mismatch sites block `100%` of Rust CI jobs and container builds.

**Recommendation:**
```rust
let decoding_key = cached_decoding_key(&jwk)?;
```
Update both `apps/auth-api/src/services/tokens.rs:521` and `apps/auth-api/src/services/tokens.rs:550` and re-run `cargo test --locked --all-targets --all-features`.

---

#### [F-004] [STILL-OPEN] MFA (`TOTP` / `WebAuthn`) Policy Flag Exposed Without Enrollment or Step-Up Verification

| Field | Value |
| --- | --- |
| **Severity** | **6 — MEDIUM** |
| **Checklist Items** | `BE-102`, `KV-126` |
| **Pillar** | Authentication (-2.0 pts) |
| **Category** | Multi-Factor Authentication & Step-Up Authorization |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/oidc_provider.rs:389`, `apps/auth-api/src/services/control_plane.rs:79` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
Environments expose `require_mfa: bool` and `oidc_provider.rs:389` checks `amr` for `"mfa" | "totp" | "webauthn" | "otp"`, yet no TOTP (`RFC 6238`) enrollment/challenge, WebAuthn/Passkey flow, or recovery codes exist. Enabling `require_mfa` locks out password users, and privileged control-plane actions (creating/rotating `sk_live_*` keys, registering webhooks) have no step-up MFA option.

**Rule 5b Validation Gate:**
- **Reachability:** Control-plane admin enables `require_mfa` → password sessions fail `evaluate_sso_policy` with no MFA enrollment route; control-plane credential mutations accept single-factor developer cookies. Verdict: **REACHABLE**.
- **Math / State-Bounds:** Compromise of a single developer password yields full `sk_live_*` issuance and webhook registration access.

**Recommendation:**
Implement encrypted TOTP (`RFC 6238`) enrollment + hashed recovery codes (and WebAuthn/Passkeys) wired into both `require_mfa` environments and sensitive control-plane credential/webhook mutations.

---

#### [F-021] [NEW] Redis Rate Limiter Fails Closed (`503`) Across All API Routes on Redis Outage

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `FV-057`, `LM-059`, `KV-100` |
| **Pillar** | Reliability (-1.5 pts) |
| **Category** | Availability & Graceful Degradation |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/middleware/rate_limit.rs:120-122,139-157` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
`middleware/rate_limit.rs:139-141` and `:156` return `503 Service Unavailable` whenever `state.redis.get()` or the Lua script invocation fails. Because `api_rate_limit_layer`, `auth_rate_limit_layer`, and `sensitive_rate_limit_layer` wrap `/v1/control/*`, `/v1/client/*`, `/v1/backend/*`, `/oauth2/*`, `/auth/*`, `/users/*`, and `/sessions/*`, any transient Redis failover turns into a **100% platform outage** — contradicting `docs/disaster-recovery.md` §1 (which guarantees graceful degradation on Redis outage). Additionally, missing `ConnectInfo<SocketAddr>` at line `120` returns `503` unconditionally.

**Recommendation:**
Add a bounded in-memory fallback limiter (per-pod conservative bucket) when Redis is unreachable — recording `auth_redis_errors_total` and emitting a structured warning — while keeping fail-closed behavior optional ONLY for `sensitive_rate_limit_layer` (OTP/reset/credential rotation) if desired.

---

#### [F-007] [PARTIALLY-FIXED] Unpatched Registration Enumeration Sibling (`authentication.rs`) & SMTP Timing Skew

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `BE-099`, `KV-051` |
| **Pillar** | Security (-0.5 pt) · Authentication (-1.0 pt) |
| **Category** | Authentication / Information Disclosure |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/authentication.rs:44-46`, `apps/auth-api/src/services/client_auth.rs:252-261,305-312` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Partially Fixed |

**Description:**
1. Sibling sweep caught that legacy `register_user` (`authentication.rs:44-46`, exposed at `POST /auth/register` and `POST /api/auth/register`) still returns `Err(AppError::EmailAlreadyExists)` before hashing.
2. In `client_auth.rs:252-261`, existing emails return immediately after Argon2id without an SMTP round-trip, whereas new emails await `send_branded_otp_email().await?` (line `305`), introducing a ~200–500ms timing side-channel when SMTP is configured.

**Recommendation:**
Align `authentication.rs:44-46` with `client_auth.rs`'s uniform response, and dispatch registration emails through a bounded `tokio::sync::mpsc` worker (or send an informational "account already exists" notice) so response timing is indistinguishable.

---

#### [F-012] [PARTIALLY-FIXED] Live Staging PITR Restore & Chaos Drills Unexecuted

| Field | Value |
| --- | --- |
| **Severity** | **5 — MEDIUM** |
| **Checklist Items** | `DEP-049`, `LM-054`, `LM-059`, `KV-100` |
| **Pillar** | Reliability (-0.5 pt) |
| **Category** | Disaster Recovery Verification |
| **Language** | Operations |
| **Files** | `docs/disaster-recovery.md` *(K8s `ingress.yaml:72-99` routes FIXED in `132b24a`)* |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Partially Fixed |

**Description:**
`ingress.yaml` now routes `/v1`, `/oauth2`, `/.well-known`, and `/api` cleanly. However, live staging execution evidence for PostgreSQL PITR restore (verifying `SIGNING_KEY_KEK` decryption after restore), Redis outage degradation, and `webhook-worker` `SIGKILL` recovery has not yet been recorded.

**Recommendation:**
Execute and attach dated staging drill sign-offs in `docs/disaster-recovery.md` during Phase 11 cloud deployment.

---

#### [F-014] [PARTIALLY-FIXED] Per-Request PostgreSQL `signing_keys` Query on Every JWT Verification

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `BE-062` |
| **Pillar** | Performance (-1.0 pt) |
| **Category** | Token Verification Latency & DB Load |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/tokens.rs:291,332,434,516,545,689-713` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Partially Fixed |

**Description:**
`cached_decoding_key` (`tokens.rs:689`) caches EC `DecodingKey` construction by `(x, y)`, eliminating repeated EC point decoding, but every JWT verification still queries PostgreSQL `signing_keys` by `kid` (`find_verifiable_signing_key_by_kid`).

**Recommendation:**
Cache `kid → (EcPublicJwk, expires_at)` in memory for 30–60 seconds with explicit invalidation on `rotate_signing_key` / `revoke_signing_key` (propagated across pods via Redis pub/sub or short TTL).

---

#### [F-022] [NEW] Secondary Email Verification Returns `500` on Verified-Email Conflict & Unused Rate-Limit Crates

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `BE-039`, `SC-017` |
| **Pillar** | Security (-0.5 pt) |
| **Category** | Error Handling & Dependency Hygiene |
| **Language** | Rust |
| **Files** | `apps/auth-api/src/services/identity_advanced.rs:199-211`, `apps/auth-api/Cargo.toml:90-91` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Open |

**Description:**
1. Removing the pre-check in `add_user_email` (`identity_advanced.rs:157-168`) causes `verify_user_email` (`lines 199-211`) to hit PostgreSQL's partial unique index (`23505`) if another user already verified that email, returning `500 DATABASE_ERROR` instead of a clean `422 VALIDATION_ERROR`.
2. `tower_governor` and `governor` remain in `apps/auth-api/Cargo.toml:90-91` despite being completely replaced by the custom Redis Lua limiter in `c0ca048`.

**Recommendation:**
Map SQLSTATE `23505` in `verify_user_email` to `AppError::Validation("Unable to verify this email address".to_string())`, and remove unused `tower_governor` and `governor` from `Cargo.toml` / `Cargo.lock`.

---

#### [F-008] [PARTIALLY-FIXED] Missing Inline `# vX.Y.Z` Comments on SHA-Pinned GitHub Actions

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `DEP-067` |
| **Pillar** | Security (-0.5 pt) |
| **Category** | CI/CD Supply Chain Maintainability |
| **Language** | YAML |
| **Files** | `.github/workflows/ci.yml`, `.github/workflows/security.yml`, `.github/workflows/release.yml` |
| **PoC Evidence Tier** | `[PoC-PROSE]` |
| **Status** | Partially Fixed |

**Description:**
`/metrics` isolation and immutable 40-hex SHA pinning + `.github/dependabot.yml` are now implemented (`132b24a`, `61cdbcb`). Per `DEP-067`, SHA-pinned `uses:` lines should carry inline `# vX.Y.Z` version comments (in addition to `docs/action-pins.json`) so reviewers and Dependabot can verify tag intent directly in workflow diffs.

---

#### [F-017] [STILL-OPEN] No Field-Level PII Encryption or Formal PII Data Inventory

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `PC-001`, `PC-002`, `PC-006` |
| **Pillar** | Privacy (-1.0 pt) |
| **Category** | Data Protection & Governance |
| **Language** | Documentation / Infrastructure |
| **Files** | `apps/auth-api/migrations/001..015` |
| **Status** | Open |

**Recommendation:**
Publish `docs/privacy-and-data-inventory.md` documenting every PII column, purpose, retention period (`DATA_RETENTION_DAYS`), and required Phase 11 KMS/TDE storage encryption.

---

#### [F-018] [STILL-OPEN] Missing Privacy Policy, Terms of Service & GDPR/SOC 2 Compliance Mapping

| Field | Value |
| --- | --- |
| **Severity** | **4 — LOW** |
| **Checklist Items** | `PC-005`, `PC-015`, `PC-016`, `PC-018`, `PC-024` |
| **Pillar** | Privacy (-1.0 pt) |
| **Category** | Regulatory Compliance |
| **Language** | Documentation |
| **Files** | `docs/` (absent) |
| **Status** | Open |

**Recommendation:**
Publish Privacy Policy, Terms of Service, and a SOC 2 / GDPR control mapping linked from the Hosted Auth Portal and Control Plane sign-up flows before public launch (Phase 12).

---

### Remaining Low-Severity Findings (Severity 3 — Tracked Inline)

- **`[F-015]` (Sev 3 — Performance -1.0 pt | `FV-044`):** Execute live staging k6 load, burst, and 2h soak baselines (`tests/load/*.js`) and record observed p95/p99 + RSS/pool metrics in `docs/performance-report.md`.
- **`[F-019]` (Sev 3 — Observability -0.25 pt | `LM-020`, `LM-021`, `LM-032`):** Wire production Alertmanager PagerDuty/Slack receivers and 90-day append-only external log sink in Phase 11.
- **`[F-020]` (Sev 3 — Developer Experience -0.5 pt | `FV-023`, `FV-033`, `LM-051`, `PC-029`, `PC-036`):** Add `.github/CODEOWNERS`, `CHANGELOG.md`, root `LICENSE`, `SECURITY.md`, CI coverage thresholds (`cargo-llvm-cov` / Vitest coverage), and `proptest`/`cargo-fuzz` targets for JWT/URL/webhook parsers.

---

## 5. Per-Checklist Audit Metrics at `61cdbcb` (Rule 8)

| # | Checklist | Scope | Items | Pass | Fail | Partial | N/A | Pass Rate (excl. N/A) |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 01–07 | On-Chain Solana (7 checklists) | OUT-OF-SCOPE | 531 | 0 | 0 | 0 | 531 | N/A |
| 08 | TypeScript Safety | IN-SCOPE | 64 | 31 | 0 | 3 | 30 | **91.2%** |
| 09 | Backend Security | IN-SCOPE | 132 | 70 | 3 | 6 | 53 | **88.6%** |
| 10 | Frontend Security | IN-SCOPE | 84 | 43 | 0 | 3 | 38 | **93.5%** |
| 11 | Supply Chain & Dependencies | IN-SCOPE | 62 | 30 | 0 | 4 | 28 | **88.2%** |
| 12 | Secrets & Key Management | IN-SCOPE | 53 | 27 | 0 | 4 | 22 | **87.1%** |
| 13 | Deployment & Infrastructure | IN-SCOPE | 96 | 36 | 2 | 6 | 52 | **81.8%** |
| 14–15 | Python / Other Languages | OUT-OF-SCOPE | 170 | 0 | 0 | 0 | 170 | N/A |
| 16 | Formal Verification & Testing | IN-SCOPE | 73 | 28 | 4 | 4 | 37 | **77.8%** |
| 17 | Logging, Monitoring & IR | IN-SCOPE | 65 | 40 | 2 | 4 | 19 | **87.0%** |
| 18 | Privacy, Compliance & Change Mgmt | IN-SCOPE | 60 | 29 | 4 | 6 | 21 | **74.4%** |
| 19 | AI Agent Security | OUT-OF-SCOPE | 35 | 0 | 0 | 0 | 35 | N/A |
| 20 | Rust Off-Chain Services | IN-SCOPE | 21 | 15 | 0 | 1 | 5 | **93.8%** |
| **Total In-Scope** | **10 In-Scope Checklists** | **100% Evaluated** | **710** | **349** | **15** | **41** | **305** | **86.2%** *(up from 77.0%)* |

### Known Vector Summary (`KV-001..KV-140`)

- **Out-of-Scope / Feature-Absent:** 73 vectors (`65` on-chain/AI + `8` feature-absent off-chain: MongoDB, BaaS, XXE, GraphQL, WebSockets, postMessage, user CSS, 3rd-party CDN scripts, comment-triggered GHA).
- **In-Scope Evaluated (67 vectors):** **59 PASS**, **2 FAIL** (`KV-051` partial sibling in `authentication.rs`, `KV-126` missing step-up MFA), **6 PARTIAL** (`KV-039/083` fail-closed Redis outage trade-off, `KV-073/092` Phase 11 DNS, `KV-087` Phase 11 Alertmanager, `KV-100` live staging PITR drill). **Notable newly CLOSED vectors since `379fa6a`:** `KV-054` (default credentials), `KV-055/082` (exposed `/metrics`), `KV-059` (clickjacking), `KV-071` (missing CSP), `KV-080` (SHA-pinned CI), `KV-094` (password length cap), `KV-096` (SSR headers), `KV-097` (dev fallback secrets), `KV-112` (secret zeroization).

---

## 6. Code Maturity Scorecard (Phase 4.5 — Scale 0–4)

| # | Category | Baseline (`379fa6a`) | Current (`61cdbcb`) | Justification |
| --- | --- | :---: | :---: | --- |
| 1 | Access Controls | 3 / 4 | **3 / 4 (Good)** | Strong tenant RBAC, RLS, scoped `sk_*`, SHA-pinned CI; deduct 1 for missing step-up MFA (`F-004`) |
| 2 | Arithmetic & Bounds | 3 / 4 | **4 / 4 (Strong)** | `overflow-checks = true`, clamped pagination/concurrency, 128-char/512-byte password cap (`F-010` FIXED) |
| 3 | Type & Memory Safety | 3 / 4 | **4 / 4 (Strong)** | Zero `unsafe`, zero `any`, redacting `Debug` + `Drop` zeroization on `AppConfig` & key material (`F-005` FIXED) |
| 4 | Input Validation | 3 / 4 | **3 / 4 (Good)** | Strict URL/SSRF/OTP/password validation; deduct 1 for `authentication.rs` enumeration & `verify_user_email` 23505 (`F-007`, `F-022`) |
| 5 | Testing | 3 / 4 | **3 / 4 (Good)** | New `security_hardening_tests.rs` + 132 existing tests; deduct 1 for active `F-023` compile break & missing CI coverage |
| 6 | **Fuzzing & Property Tests** | 0 / 4 | **0 / 4 (Absent)** | **Priority Remediation:** Zero `cargo-fuzz` or `proptest` targets (`F-020`) |
| 7 | Error Handling & DoS Resilience | 2 / 4 | **3 / 4 (Good)** | `CatchPanicLayer` + `unwind` + bounded Argon2id + shared HTTP clients + SMTP timeouts (`F-002`, `F-011`, `F-013` FIXED); deduct 1 for Redis fail-closed `503` (`F-021`) |
| 8 | Deployment & Secret Operations | 2 / 4 | **3 / 4 (Good)** | Fail-closed config, full K8s secrets & Ingress routes, `setup-env.mjs`, SHA-pinned GHA + Dependabot (`F-001`, `F-008`, `F-012` Ingress FIXED); deduct 1 for unexecuted live PITR drill |
| 9 | Monitoring & Incident Response | 3 / 4 | **3 / 4 (Good)** | Dedicated `/metrics` port `9090` + email/retention metrics + OTel/Grafana/runbooks; deduct 1 for Phase 11 Alertmanager & live DR drill sign-off |

---

## 7. Prioritized Remediation Roadmap (Post-`61cdbcb`)

### Immediate (Today — Unblock CI)
1. **`F-023` (Sev 6 — Build Blocker):** Borrow `&jwk` at `apps/auth-api/src/services/tokens.rs:521,550` and re-run `cargo test --locked --all-targets --all-features`. *(Immediately restores Rust CI and raises score from **87.0 → 89.0 / 100**.)*

### P1 — Pre-Launch Hardening (Within 1–2 Weeks)
2. **`F-021` (Sev 5 — Reliability):** Add bounded local fallback in `apps/auth-api/src/middleware/rate_limit.rs` when Redis is unavailable (emitting `auth_redis_errors_total`) so Redis failover degrades gracefully per `docs/disaster-recovery.md` §1.
3. **`F-004` (Sev 6 — Authentication):** Implement TOTP (`RFC 6238`) + hashed recovery codes (and WebAuthn/Passkeys) backing `require_mfa` and developer control-plane step-up auth.
4. **`F-007` & `F-022` (Sev 5/4 — Security/Auth):** Close legacy registration enumeration in `apps/auth-api/src/services/authentication.rs:44-46`, equalize SMTP timing on `client_auth.rs:252-312`, map SQLSTATE `23505` cleanly in `identity_advanced.rs:199-211`, and remove unused `tower_governor`/`governor` from `Cargo.toml`.
5. **Maturity Category 6 (0/4 — Priority):** Add `proptest` / `cargo-fuzz` targets for `domains::validate_redirect_uri`, `webhooks::validate_webhook_url`, `webhooks::verify_signature_header`, `rate_limit::resolve_client_ip`, and JWT decoding.

### P2 — Phase 11 & Phase 12 Launch Readiness
6. **`F-012`, `F-015`, `F-019` (Phase 11 Cloud Rollout):** Execute live staging PITR restore, Redis outage, `webhook-worker` `SIGKILL`, and k6/2h soak drills; wire Alertmanager PagerDuty/Slack receivers and 90-day append-only log storage.
7. **`F-014` & `F-008` (Hardening):** Add short-TTL `kid → EcPublicJwk` cache with invalidation on key revocation/rotation; add inline `# vX.Y.Z` comments beside SHA pins in `.github/workflows/*.yml`.
8. **`F-017`, `F-018`, `F-020` (Governance & Docs):** Add `docs/privacy-and-data-inventory.md`, Privacy Policy, ToS, `.github/CODEOWNERS`, `CHANGELOG.md`, root `LICENSE`, `SECURITY.md`, and CI coverage tracking.

---

## 8. Notes & Nitpicks (Below Severity 1)

- `apps/web/vite.config.ts:23-29`: `svelte-check` emits `config_option_deprecated_alias`; migrate `$lib`/`$api` aliases to `package.json` subpath imports before SvelteKit 4.
- `apps/auth-api/Dockerfile` vs `docker/Dockerfile.api` (and `apps/web/Dockerfile` vs `docker/Dockerfile.web`): near-duplicate Dockerfile pairs; consolidate to one canonical pair.
- `apps/auth-api/Cargo.toml:7`: `authors = ["Your Name <you@example.com>"]` placeholder string.

---

## 9. Appendices & Disclaimer

- **Toolchain Verified:** Rust `1.96.0`, Node.js `v24.11.1`, `eslint 10.9.1`, `svelte-check 4.0.0`, `vitest 5.1.1`.
- **Disclaimer:** This report reflects `auditor-skill` v7.6 automated and manual re-audit evidence through commit `61cdbcb`. It is a rigorous engineering security assessment and does not replace an independent external penetration test or formal compliance certification prior to regulated production launch.
