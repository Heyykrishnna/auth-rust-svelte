# Phase 12: Developer Docs, Dual-App Dogfooding, Metering & Launch

- **Status:** Planned
- **Prerequisites:** Phase 11 completed (staging & production pipelines verified)
- **Primary Targets:** `docs-site/` (or `apps/docs/`), `packages/cli/`, `examples/`, `apps/auth-api/src/services/metering.rs`

---

## Objective

Prove the platform end-to-end by **dogfooding it across multiple real, independent applications (e.g., Ferrum and Elysiar)**, publish complete developer documentation and CLI tooling, establish usage metering/quotas, and execute a controlled beta-to-public production launch so any new project can adopt the platform cleanly from day one.

---

## 1. Dual-App Real-World Dogfooding (`Ferrum` + `Elysiar`)

Validate the entire product vision against real independent services before external launch:

- Register **Ferrum** (e.g., SvelteKit + Postgres app) and **Elysiar** (e.g., Next.js + Postgres app) purely via the Developer Dashboard—zero manual DB hacks or custom auth code in either app.
- Verify the complete real-world lifecycle:
  1. User signs up on Ferrum via Hosted Auth Portal -> Ferrum receives `user.created` + `membership.created` webhook and upserts its local `users` table keyed by `sub` + `version`.
  2. User opens Elysiar -> OIDC SSO skips password prompt (respecting `enrollment_policy`), issues Elysiar-scoped tokens, and Elysiar syncs the user via JIT claims + webhook.
  3. User updates profile / email or creates an organization -> both joined apps receive strictly ordered, HMAC-verified webhook updates; an out-of-order replay is safely ignored via `version` guard.
  4. Simulate 10 minutes of Elysiar downtime during user updates -> Elysiar runs the SDK reconciliation helper (`/v1/backend/users?updated_since=...`) on recovery and reaches 100% DB parity with zero manual intervention.
  5. Test local logout (Ferrum only) vs. global logout (revokes IdP `sso_session`, Ferrum, and Elysiar).

---

## 2. Public Documentation Website & Interactive Reference

Publish versioned documentation covering every developer workflow:

- **Getting Started:** Architecture overview (Global Identity vs App Membership, Dev vs Prod), 10-minute Quickstart, Application Registration & API Keys (`pk_*` / `sk_*`).
- **Authentication & SSO:** Hosted OIDC SSO guide, Embedded SDK guide (and explicit trade-offs regarding cross-domain SSO), Social OAuth setup (Google/GitHub), Session Management & Local/Global Logout.
- **Service Synchronization Guide (Flagship Section):**
  - The 4-Layer Sync Pattern (Webhooks + JIT Claims + Cursor Reconciliation + Revocation).
  - Reference SQL schemas & idempotent upsert snippets (PostgreSQL/SQLx, Prisma, Drizzle) keying on `sub` / `app_user_id` + `version`.
  - Webhook signature verification, retry/DLQ handling, and downtime recovery playbook.
- **Framework Guides:** SvelteKit 5, React, Next.js (App Router & Edge Middleware), and Generic OIDC (Rust/Go/Python/Mobile).
- **Organizations & RBAC:** Multi-org flows, invitations, custom roles/permissions, reading org claims in backend APIs.
- **Interactive OpenAPI Reference:** Generated directly from `auth-api` contracts for Client, Backend, OIDC, and Control Plane APIs + full Error Code Catalog + Security Best Practices.

---

## 3. Developer CLI & Onboarding Tooling (`packages/cli`)

- Lightweight CLI (`npx @secure-auth/cli ...`) focused on fast project integration and sync testing:
  - `init`: scaffolds SDK config, callback handler, middleware, and webhook sync handler into a SvelteKit/Next.js project.
  - `webhooks listen --forward-to http://localhost:3000/api/webhooks/auth`: local webhook forwarding/testing with valid HMAC signatures.
  - `verify`: validates live env vars, JWKS reachability, redirect URI alignment, and webhook signature round-trip before deployment.

---

## 4. Usage Metering, Quotas & Billing Readiness

- **Usage Metering:** Track per-workspace / per-environment Monthly Active Users (MAU), active organizations, API request volume, and webhook deliveries from Phase 09 rollups.
- **Plan & Quota Guardrails:** Configurable per-workspace plan limits & feature entitlements (`max_environments`, `max_mau`, `custom_domains`, `webhooks_enabled`, `organizations_enabled`) with soft-warning alerts before hard rate/quota limits.
- **Billing Event Records:** Append-only billing usage ledger ready for future Stripe/subscription processor integration without coupling auth hot paths to billing outages.

---

## 5. Product Operations & Launch Readiness

- Public Status Page connected to live synthetic probes (OIDC authorize, JWKS, Client API, Webhook worker latency).
- Release notes, SDK semver changelog, deprecation policy, and incident communication templates.
- External developer beta cohort -> gather onboarding friction metrics -> remediate -> Public v1.0 release checklist sign-off.

---

## Deliverables

- Live dogfooded multi-app integration (`Ferrum` + `Elysiar`) passing full SSO + 4-layer sync verification
- Public Documentation Site + Interactive OpenAPI Reference
- Developer CLI (`@secure-auth/cli`) & Starter Templates
- Usage Metering, Quotas & Entitlements service
- Public Status Page, Changelog, and v1.0 Release Sign-Off

---

## Acceptance Criteria

- [ ] Two independent production-like applications (`Ferrum` and `Elysiar`) run live on the platform with zero custom auth backend code, working OIDC SSO, and verified 4-layer database synchronization (webhooks + JIT + reconciliation + revocation).
- [ ] A new developer following only the public Quickstart + CLI can register an app, authenticate users, and sync user records to their local DB in under 15 minutes.
- [ ] Interactive OpenAPI docs match the live `/v1/*` and `/oauth2/*` responses with automated contract verification.
- [ ] Usage metering, rate limits, and quota alerts operate accurately without adding latency to authentication hot paths.
- [ ] Status page, monitoring alerts, on-call runbooks, and external beta feedback items are resolved and signed off for v1.0 launch.

