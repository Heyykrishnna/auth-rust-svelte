# Phase 06: TypeScript SDKs, Framework Integrations & Service Sync Kits

- **Status:** Planned
- **Prerequisites:** Phase 04 (OIDC + Hosted Portal) & Phase 05 (Webhooks + Backend Reconciliation API) completed
- **Primary Targets:** `packages/core/`, `packages/svelte/`, `packages/react/`, `packages/next/`, `examples/`

---

## Objective

Deliver a Clerk-grade developer integration experience so adding authentication **and** database synchronization to any new SvelteKit, React, or Next.js project takes minutes—with zero custom password hashing, OAuth state plumbing, token refresh boilerplate, or hand-rolled webhook verification.

---

## 1. Core SDK (`@secure-auth/core`)

Runtime-agnostic TypeScript package (strict mode, Web Standard `fetch` + **Web Crypto API** via `jose` so it runs identically in browsers, Node.js, Next.js Edge Runtime, Cloudflare Workers, and Deno):

- **Browser / Client Module (`@secure-auth/core/client`):**
  - `createAuthClient({ publishableKey, issuerUrl, redirectUri })`
  - **Hosted OIDC SSO Helpers (Primary Multi-App Mode):** `signInWithRedirect()`, `handleRedirectCallback()` (automatic PKCE `S256` generation, `state` & `nonce` storage/verification, code exchange), `signOut({ global?: boolean })`.
  - **Direct Client API Helpers (App-Local Embedded Mode):** `signInWithPassword()`, `signUp()`, `verifyOtp()`, `requestPasswordReset()`, `refreshSession()`, `getSession()`, `getUser()`.
  - Normalized typed error classes (`AuthApiError`, `TokenExpiredError`, `RateLimitError`, `NetworkError`), timeout/retry policies for idempotent reads, automatic single-flight token refresh (prevents parallel refresh races across concurrent requests/tabs).
- **Server & Verification Module (`@secure-auth/core/server`):**
  - `createAuthServer({ secretKey, publishableKey, issuerUrl, audience })`
  - **Stateless Local JWT Verification (`verifyAccessToken` / `verifyIdToken`):** Automatic JWKS fetching with in-memory TTL caching, `kid` miss refresh (rate-limited for key rotation), strict `iss`, `aud`, `exp`, `nbf`, and `typ` verification using Web Crypto.
  - **Typed Backend Sync & Management Client:** Wrappers for `/v1/backend/users`, `/v1/backend/events`, `/v1/backend/sessions/:id/revoke`, metadata updates, and async iterator `iterateUsersUpdatedSince(cursor)` for effortless reconciliation scripts.
- **Service Sync & Webhook Module (`@secure-auth/core/webhooks`):**
  - `verifyWebhookSignature({ payload, headers, secret, toleranceSecs })` — constant-time HMAC-SHA256 verification using Web Crypto (Edge-safe), timestamp skew enforcement, dual-secret rotation support.
  - `createWebhookRouter(handlers)` — strongly typed dispatcher (`onUserCreated`, `onUserUpdated`, `onUserDeleted`, `onMembershipCreated`, `onSessionRevoked`, etc.) with built-in idempotency hook (`isEventProcessed(eventId)`) and version-guard helper (`shouldApplyVersion(localVersion, incomingVersion)`).
  - **JIT Sync Helper (`syncUserFromClaims`)** — lightweight helper turning verified JWT claims into an idempotent upsert payload keyed by immutable `sub` / `app_user_id`.

---

## 2. SvelteKit & Svelte 5 SDK (`@secure-auth/svelte`)

First-class integration tailored to Svelte 5 runes and SvelteKit SSR:

- **Client & Components:**
  - `<AuthProvider>` context with Svelte 5 reactive `$state` / `$derived` stores (`useAuth()`, `useUser()`, `useSession()`).
  - `<SignIn />` (supports `mode="redirect"` for Hosted SSO or `mode="embedded"`), `<SignUp />`, `<UserButton />` (avatar, active org, manage account link, local/global sign-out), `<SignedIn>`, `<SignedOut>`, `<ProtectedRoute>`.
- **SvelteKit Server Integration (`@secure-auth/svelte/server`):**
  - `createAuthHandle({ ... })` for `hooks.server.ts` — validates session cookie / Bearer token via JWKS, populates `event.locals.auth`, handles transparent server-side refresh safely, protects routes via `requireAuth()` / `requirePermission()`.
  - Ready-to-mount route handlers: `createOidcCallbackHandler()` and `createWebhookEndpointHandler({ secret, handlers })` for `+server.ts`.
  - SSR-safe hydration without exposing raw refresh tokens or secret keys to client payloads.

---

## 3. React SDK (`@secure-auth/react`)

- `<AuthProvider>`, `useAuth()`, `useUser()`, `useSession()`, `usePermissions()`.
- Drop-in accessible components: `<SignIn />`, `<SignUp />`, `<UserButton />`, `<SignedIn>`, `<SignedOut>`, `<Protect permission="...">`.
- Support for Hosted OIDC SSO redirect flow + embedded mode.

---

## 4. Next.js App Router SDK (`@secure-auth/next`)

- **Edge-Compatible Middleware (`authMiddleware`):** Uses `@secure-auth/core/server` (Web Crypto JWKS verification) so it runs in both **Next.js Edge Runtime** and **Node.js Runtime** with zero Node-only crypto crashes.
- **Server Components & Server Actions (`@secure-auth/next/server`):** `auth()`, `currentUser()`, `protect()` helpers reading verified request context.
- **Route Handlers:** `createCallbackRouteHandler()` (OIDC code exchange -> `HttpOnly` app cookies) and `createWebhookRouteHandler()` (verifies raw body HMAC signature and dispatches typed sync handlers).
- **Client Components:** Re-exports `<AuthProvider>`, hooks, and UI components cleanly separated with `"use client"` boundaries.

---

## 5. Package Engineering & Developer Experience

Target integration experience in any new project:

```ts
// 1. Auth setup (e.g., SvelteKit hooks.server.ts)
import { createAuthHandle } from "@secure-auth/svelte/server";
export const handle = createAuthHandle({
  publishableKey: env.PUBLIC_AUTH_PUBLISHABLE_KEY,
  secretKey: env.AUTH_SECRET_KEY,
  issuerUrl: env.PUBLIC_AUTH_ISSUER_URL,
});

// 2. Database sync webhook endpoint (src/routes/api/webhooks/auth/+server.ts)
import { createWebhookEndpointHandler } from "@secure-auth/svelte/server";
export const POST = createWebhookEndpointHandler({
  secret: env.AUTH_WEBHOOK_SECRET,
  handlers: {
    "user.updated": async (evt) => await db.upsertUserBySub(evt.data),
    "session.revoked": async (evt) => await cache.invalidateSession(evt.data.session_id),
  },
});
```

- Strict TypeScript (`strict: true`, `noUncheckedIndexedAccess: true`), clean `exports` maps, ESM (+ CJS where needed), bundled `.d.ts` declarations, tree-shakeable subpaths.
- Semantic versioning and Changesets / automated npm release workflow.
- Reference **Sync Starter Examples** (`examples/ferrum-sveltekit` and `examples/elysiar-nextjs`) demonstrating full OIDC SSO + local DB webhook sync + JIT upsert + reconciliation script.

---

## Required Tests & Verification Gates (Executed in Phase 06)

- **Unit & Contract Tests:** Core client/server/webhook tests including single-flight refresh concurrency, JWKS rotation cache refresh, `aud`/`iss` rejection, and HMAC signature tamper/expiry tests.
- **Edge Runtime Compatibility Test:** Verify `@secure-auth/core/server` and `@secure-auth/next` middleware execute cleanly with zero Node built-in imports.
- **SSR & Hydration Tests:** SvelteKit and Next.js integration tests verifying authenticated SSR rendering, protected route redirects, and zero secret leakage in HTML/serialized payloads.
- **Cross-App SDK E2E Test:** Browser test running `examples/ferrum-sveltekit` and `examples/elysiar-nextjs` against live `auth-api`, proving SSO login + local DB webhook sync + reconciliation backfill.

---

## Deliverables

- `@secure-auth/core`, `@secure-auth/svelte`, `@secure-auth/react`, `@secure-auth/next` workspace packages
- Two full reference apps (`examples/ferrum-sveltekit`, `examples/elysiar-nextjs`) demonstrating OIDC SSO + local DB sync
- SDK API Reference & Quickstart guides

---

## Acceptance Criteria

- [ ] A fresh SvelteKit or Next.js application can integrate working authentication, route protection, and HMAC-verified database sync webhooks solely through SDK installation and environment variables.
- [ ] `@secure-auth/core/server` and `@secure-auth/next` middleware verify asymmetric JWTs via JWKS in both Node.js and Edge runtimes with strict `aud`/`iss` checks.
- [ ] Webhook sync helpers enforce HMAC signatures, timestamp tolerances, and expose event IDs and monotonic `version` fields cleanly.
- [ ] All SDK unit, contract, SSR, and dual-app browser integration tests pass.

---

## Out of Scope

- Native mobile SDKs (iOS/Android/Flutter) — can use standard OIDC PKCE directly via Phase 04
- Organization switcher UI components (added to SDKs in Phase 08)

