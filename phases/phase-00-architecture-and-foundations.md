# Phase 00: Architecture, Decisions, Contracts & Codebase Foundations

- **Status:** Completed (pending user sign-off on `docs/adr/0003-developer-control-plane-identity.md` and brand/SDK scope before Phase 01)
- **Prerequisites:** Current repository baseline (`apps/auth-api`, `apps/web`, `packages/shared-types`)
- **Primary Targets:** `docs/`, `apps/auth-api/src/`, `apps/web/src/`, `packages/shared-types/`

---

## Objective

Lock down every foundational architectural decision, API contract, trust boundary, and codebase cleanup **before** introducing multi-tenant schema migrations. Eliminating ambiguities and existing single-tenant defects here prevents costly rework across Phases 01–12.

---

## 1. Architecture Decisions (ADRs) to Finalize

Record explicit Architecture Decision Records under `docs/adr/`:

1. **ADR-001 — Environments / Instances Model:** Formalize `applications` -> `application_environments` (`dev` and `prod` instances) as the true tenant boundary (`environment_id`) for keys, domains, memberships, sessions, organizations, roles, and webhooks.
2. **ADR-002 — Central OIDC SSO vs. Embedded SDK Auth Contract:**
   - **Hosted OIDC Authorization Code + PKCE** on the IdP origin is the canonical cross-app SSO mechanism (first-party `sso_session` cookie, resilient to third-party cookie blocking).
   - **Embedded `<SignIn />` SDK components** operate as app-local authentication unless delegating to an IdP redirect handoff.
3. **ADR-003 — Developer Control Plane Identity Strategy:** Decide and document whether `developers` use a dedicated credentials table (isolated blast radius) or dogfood global `users` with a reserved System Control Plane environment + `developer_workspaces`.
4. **ADR-004 — First-Time App Enrollment Policy:** Define per-environment `enrollment_policy` (`consent_required` default, `auto_join`, `invite_only`) when an authenticated global user arrives at a second app via SSO.
5. **ADR-005 — Social OAuth Ownership:** Define global IdP-managed social providers vs. optional per-environment `application_oauth_providers` overrides and safe email-verified account linking rules.
6. **ADR-006 — Three-Plane API & Versioning Strategy:**
   - OIDC & Discovery: `/.well-known/openid-configuration`, `/.well-known/jwks.json`, `/oauth2/*`
   - Client/Frontend API: `/v1/client/*`
   - Backend Management & Sync API: `/v1/backend/*`
   - Developer Control Plane API: `/v1/control/*`
7. **ADR-007 — Service Synchronization Contract:** Standardize the 4-layer sync contract (outbox webhooks with monotonic `version`, JIT claims, cursor reconciliation API, revocation semantics) and mandate immutable `sub` / `app_user_id` as foreign keys in consuming services.

---

## 2. Backend Codebase Audit & Pre-Migration Remediation (`apps/auth-api`)

Address concrete structural issues identified in the existing Rust codebase before layering multi-tenancy on top:

- **Fix existing session ID mismatch defect:** In `apps/auth-api/src/services/authentication.rs` and `apps/auth-api/src/services/sessions.rs`, `authentication.rs` currently generates a `session_id` for the refresh JWT while `Session::new(...)` generates a different UUID. Unify session ID generation so the persisted session ID always matches the token claim.
- **Enforce explicit PostgreSQL transactions:** Stop silently ignoring write errors (`let _ = ...`) in `create_session`, role assignment, and audit logging. Introduce a clear transaction boundary pattern (`sqlx::Transaction<'_, Postgres>`) for multi-step domain operations.
- **Refactor layering & error envelopes:** Standardize transport (`handlers`), application orchestration (`services`), pure domain rules (`domain` / `models`), and persistence (`repositories`). Adopt a single consistent JSON error envelope (`{ "error": { "code", "message", "request_id", "details" } }`) across all endpoints.
- **Eliminate fragile Postgres ENUM migration bottlenecks:** Plan transition of `oidc_provider` and `auth_audit_event` from Postgres ENUMs to constrained `TEXT` identifiers so new providers and audit events do not require DDL locks.
- **Plan dynamic CORS & route-scoped CSRF:** Design replacement of startup-static `CorsLayer` (`apps/auth-api/src/routes/mod.rs`) with request-time dynamic origin validation backed by cached `application_domains`, and replace suffix-based CSRF path checks (`apps/auth-api/src/middleware/csrf.rs`) with explicit router-group middleware.
- **Remove legacy duplicated route mounts:** Plan deprecation/removal of duplicate `/auth` + `/api/auth`, `/users` + `/api/users`, `/sessions` + `/api/sessions` in favor of versioned route planes.
- **Introduce internal domain event abstraction:** Define strongly-typed domain event structs that services emit inside database transactions (consumed by audit logging and Phase 05 outbox).

---

## 3. Frontend & Package Foundations (`apps/web`, `packages/`)

- **Split Frontend Surfaces Architecturally:** Define clean separation between:
  1. **Hosted Auth Portal** (end-user login, signup, OTP verify, password reset, OIDC consent, account security — served on the IdP origin).
  2. **Developer Control Plane Dashboard** (developer workspace, apps, environments, keys, domains, users, webhooks, analytics).
- **Audit `apps/web`:** Catalog reusable Svelte 5 components (`Button`, `Card`, `Input`, `Modal`, `Navbar`, `Toast`, `Badge`) and design tokens for reuse across both surfaces.
- **Clean up `packages/shared-types`:** Remove duplicate DTO definitions (`AuthUser` vs `UserProfile`), standardize permission naming (`resource.action`), and establish the OpenAPI-to-TypeScript contract workflow for future SDK packages.
- **Define package scope & monorepo tooling:** Finalize npm scope (e.g., `@secure-auth/*` or chosen brand scope), build pipeline (`tsup`/ESM+CJS export maps), and workspace scripts.

---

## 4. Security Threat Model & Engineering Standards

- Produce a written threat model (`docs/threat-model.md`) covering:
  - Cross-tenant data leakage (SQL, cache keys, JWT `aud` confusion, CORS misconfiguration)
  - OIDC attacks (code interception, redirect URI manipulation, CSRF/state fixation, mix-up, nonce replay)
  - Credential theft (`sk_*` leakage, signing key compromise, refresh token replay)
  - Account takeover (unverified email linking, credential stuffing, OTP brute force)
  - Webhook SSRF and event spoofing / out-of-order replay
- Verify repository formatting, linting, and CI gates (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `svelte-check`, `eslint`, `vitest`).

---

## Deliverables

- `GLOSSARY.md` (canonical domain vocabulary)
- `docs/architecture.md` updated to the Three-Plane Multi-Tenant + OIDC SSO + Service Sync architecture
- `docs/adr/0001-application-environments-as-tenant-boundary.md` through `docs/adr/0007-four-layer-service-synchronization.md`
- `docs/er-diagram.md` (canonical target schema) and `docs/sso-sequences.md` (login, SSO, consent, refresh, local/global logout, webhook sync)
- `docs/api-specification.md` (initial `v1` contract outline for all planes and outbound webhooks)
- `docs/threat-model.md`
- Remediated existing single-tenant defects in `apps/auth-api` (session ID consistency, transaction helpers, unified error envelope, domain event contract) and cleaned `packages/shared-types` with passing baseline tests

---

## Acceptance Criteria

- [x] All 7 ADRs (`0001` through `0007`) are written, reviewed, and internally consistent with zero unresolved contradictions between Image 1 ER model, Image 2 SSO flow, and the 4-layer sync contract.
- [x] The existing session ID mismatch bug in `apps/auth-api` is fixed and covered by a regression test proving stored `Session.id` matches the refresh token `session_id` claim.
- [x] A unified API error envelope (`ApiErrorEnvelope`) and PostgreSQL transaction helper (`repositories::db`) exist in `apps/auth-api`.
- [x] Frontend architecture cleanly defines the boundary between the Hosted Auth Portal and the Developer Control Plane Dashboard.
- [x] `packages/shared-types` has zero duplicate user interfaces or inconsistent permission string conventions.
- [x] Existing Rust (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`) and web (`npm run check`, `npm run test`, `npm run lint`, `npm run build`) checks pass cleanly.

---

## Out of Scope

- Creating multi-tenant database tables (Phase 01)
- Issuing asymmetric JWTs or OIDC endpoints (Phases 03–04)
- Publishing external SDK packages (Phase 06)

