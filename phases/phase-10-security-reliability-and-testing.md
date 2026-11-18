# Phase 10: Security Hardening, Load Testing & Disaster Recovery

- **Status:** Planned
- **Prerequisites:** Phases 01–09 completed (including per-phase tenant-isolation suites)
- **Primary Targets:** `apps/auth-api/tests/`, `tests/load/`, `tests/security/`, `docs/security-assessment.md`

---

## Objective

Subject the complete multi-tenant Identity & Sync Platform to adversarial security verification (OWASP ASVS), cross-tenant penetration tests, realistic high-concurrency load/soak testing, and chaos/failover disaster recovery drills before production deployment.

---

## 1. End-to-End Tenant Isolation Audit Suite

Consolidate and expand every phase's isolation tests into an automated **Cross-Tenant Red-Team Suite** running in CI against two workspaces, two applications (`Ferrum`, `Elysiar`), and four environments (`dev`/`prod` each):

- Cross-environment user profile, metadata, and `app_session` access attempts.
- Cross-application JWT `aud` substitution and `id_token` replay attempts.
- Cross-environment `sk_test_*` / `sk_live_*` key misuse and scope-escalation probes.
- Cross-organization membership, invitation, and role-elevation probes.
- Cross-application webhook fan-out, delivery log inspection, and reconciliation cursor probes.
- Cross-environment analytics and audit log access probes.

---

## 2. Adversarial Security Testing (OWASP ASVS Level 2/3)

- **Authentication Abuse & Credential Stuffing:** Verify per-IP + per-email rate limiting, lockout durations, constant-time OTP/hash checks, and account-enumeration resistance (timing + response parity).
- **JWT & OIDC Attacks:** Test `alg: none`, HS256-with-public-key algorithm confusion, unknown/revoked `kid`, expired/not-before tokens, wrong `iss`/`aud`, PKCE downgrade (`plain`), auth code replay, redirect URI parser differentials (`https://good.com@evil.com`, path traversal, Unicode tricks), and CSRF on OIDC callbacks.
- **Webhook SSRF & Spoofing:** Test DNS rebinding to `169.254.169.254`, IPv6-mapped IPv4 (`::ffff:127.0.0.1`), 301/302 redirect chains to internal hosts, slow-loris hanging endpoints (timeout enforcement), oversized response bodies, and HMAC replay outside tolerance window.
- **Web & Supply-Chain Security:** XSS/CSP verification on Hosted Portal and Dashboard, `HttpOnly`/`Secure`/`SameSite` cookie flag assertions, SQL injection checks, `cargo audit` / `cargo deny`, `npm audit`, Gitleaks secret scanning, and SAST.

---

## 3. Performance, Load & Soak Testing (`tests/load/`)

Build reproducible k6 (or Goose) load scripts with explicit SLO budgets:

- **Workloads:**
  - Concurrent sign-in (Argon2id CPU saturation — verify bounded Tokio blocking threadpool prevents starving fast JWT/JWKS/session endpoints!).
  - Concurrent registration & OTP verification.
  - High-throughput token refresh & session validation (including concurrent refresh races on the same family).
  - JWKS & `/oauth2/userinfo` read spikes.
  - Burst webhook fan-out (e.g., 10,000 events into `outbox_events` with mixed healthy/slow/failing consumer endpoints).
  - Sustained soak test (multi-hour run verifying zero memory leaks, zero leaked DB connections, stable p99 latency).
- **Define & Verify Explicit Performance Budgets:** e.g., stateless JWKS/session validation p99 < 25ms, token refresh p99 < 100ms, Argon2id sign-in isolated to dedicated blocking pool without degrading health/JWKS availability.

---

## 4. Reliability, Chaos & Disaster Recovery Drills

- **Redis Failure Mode Testing:** Simulate Redis outage -> verify graceful degradation strategy (rate limits fail safe or fall back cleanly, sessions fall back to PostgreSQL where designed, clear metrics/alerts emitted without panicking pods).
- **Database Failover & Connection Pool Recovery:** Simulate primary DB restart/failover -> verify SQLx pool recovers cleanly without pod crash loops.
- **Worker Crash & Restart Safety:** `SIGKILL` `webhook_worker` mid-batch -> verify `FOR UPDATE SKIP LOCKED` leases recover cleanly with zero lost events and idempotent delivery.
- **Graceful Shutdown (`SIGTERM`):** Verify in-flight HTTP requests and active DB transactions drain cleanly before exit.
- **Backup & Point-In-Time Recovery (PITR) Drill:** Execute full backup -> destructive test -> restore into clean environment -> verify schema, encrypted `signing_keys` decryption, and active sessions/users integrity.
- **Key Compromise Runbook Drill:** Execute emergency `signing_keys` revocation + immediate rotation and verify compromised key is dropped from JWKS and rejected immediately.

---

## Deliverables

- Automated Cross-Tenant Red-Team & OWASP ASVS security test suites (`apps/auth-api/tests/security_*`)
- Reproducible load/soak test suite (`tests/load/`) + baseline benchmark report (`docs/performance-report.md`)
- Security Assessment & Vulnerability Remediation Log (`docs/security-assessment.md`)
- Verified Disaster Recovery & Key-Compromise Runbooks (`docs/disaster-recovery.md`)

---

## Acceptance Criteria

- [ ] 100% of cross-tenant red-team isolation tests and OWASP ASVS security regression tests pass with zero critical or high unresolved vulnerabilities.
- [ ] Argon2id hashing runs on a bounded blocking pool and does not starve JWKS, token refresh, or health endpoints under sign-in load spikes.
- [ ] Load and soak tests meet defined p95/p99 latency and zero-leak criteria.
- [ ] Redis outage, worker mid-batch crash, graceful `SIGTERM` shutdown, PITR database restore, and emergency signing-key revocation drills have all been executed and verified.

---

## Out of Scope

- Cloud production rollout & GitOps wiring (Phase 11)

