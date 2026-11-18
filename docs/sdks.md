# TypeScript SDKs & Framework Integrations Reference

This guide documents the four workspace SDK packages introduced in Phase 06 and the two reference sync starter applications (`examples/ferrum-sveltekit` and `examples/elysiar-nextjs`).

| Package | Runtime Targets | Primary Use Case |
| --- | --- | --- |
| `@secure-auth/core` | Browser, Node.js 22+, Edge Runtime, Cloudflare Workers, Deno | Web Standard (`fetch` + `crypto.subtle`) OIDC PKCE client, stateless ES256 JWKS verification, Backend Sync API client, and HMAC-SHA256 webhook router |
| `@secure-auth/svelte` | Svelte 5 (Runes) + SvelteKit 2 SSR | Reactive auth context, accessible Svelte 5 UI components, `hooks.server.ts` auth handle, OIDC callback, and webhook `+server.ts` handlers |
| `@secure-auth/react` | React 18 & 19 | `<AuthProvider>`, `useAuth`, `useUser`, `useSession`, `usePermissions`, `<SignIn>`, `<SignUp>`, `<UserButton>`, `<SignedIn>`, `<SignedOut>`, `<Protect>` |
| `@secure-auth/next` | Next.js 14/15+ App Router (Edge & Node.js) | Edge-compatible `authMiddleware`, App Router `auth()`, `currentUser()`, `protect()`, OIDC callback & webhook route handlers, and `"use client"` UI re-exports |

All four packages rely exclusively on Web Standards (`fetch`, `Request`, `Response`, `Headers`, and `crypto.subtle`) with **zero external runtime dependencies** and **zero Node.js built-in imports**.

---

## 1. `@secure-auth/core`

### Subpath Exports

- `@secure-auth/core` — Aggregated exports (client, server, webhooks, errors, encoding, types).
- `@secure-auth/core/client` — Browser/client auth and Hosted OIDC PKCE helpers.
- `@secure-auth/core/server` — Stateless ES256 JWKS verification and Backend Sync API client.
- `@secure-auth/core/webhooks` — Constant-time HMAC-SHA256 webhook verification, router, version guard, and JIT claim projection helpers.

### Client Quickstart (`@secure-auth/core/client`)

```ts
import { createAuthClient } from '@secure-auth/core/client';

const authClient = createAuthClient({
  issuerUrl: 'https://auth.example.com',
  publishableKey: 'REPLACE_WITH_PUBLISHABLE_KEY',
  clientId: 'REPLACE_WITH_OIDC_CLIENT_ID',
  redirectUri: 'https://ferrum.example.com/auth/callback'
});

const authorizeUrl = await authClient.signInWithRedirect({
  scope: 'openid profile email offline_access'
});
window.location.assign(authorizeUrl);
```

On the callback route:

```ts
const tokens = await authClient.handleRedirectCallback(window.location.href);
```

Key guarantees:

- Generates a cryptographically random PKCE `code_verifier`, `S256` `code_challenge`, `state`, and `nonce`.
- Verifies `state` and ID token `nonce` before accepting tokens.
- Deduplicates concurrent `refreshSession()` calls into a single in-flight network request (single-flight refresh).
- Normalizes API error envelopes into typed errors: `AuthApiError`, `TokenExpiredError`, `TokenVerificationError`, `RateLimitError` (with `retryAfterSecs`), `NetworkError`, and `WebhookSignatureError`.

### Server & Backend Sync Quickstart (`@secure-auth/core/server`)

```ts
import { createAuthServer } from '@secure-auth/core/server';

const authServer = createAuthServer({
  issuerUrl: 'https://auth.example.com',
  audience: 'ferrum',
  secretKey: process.env.AUTH_SECRET_KEY
});

const claims = await authServer.verifyAccessToken(bearerToken);

for await (const user of authServer.iterateUsersUpdatedSince('2026-01-01T00:00:00Z', {
  pageSize: 100
})) {
  await upsertLocalUser(user);
}
```

Key guarantees:

- Enforces `alg === 'ES256'`, expected `typ` (`at+jwt` for access tokens, `JWT` for ID tokens), `iss`, `aud`, `exp`, and `nbf`.
- Rejects symmetric algorithm confusion (`HS256`, `none`) and tampered payloads.
- Caches JWKS with TTL + ETag revalidation and performs rate-limited refresh on unknown `kid` for seamless zero-downtime key rotation.
- Exposes typed wrappers for `/v1/backend/users`, `/v1/backend/users/:appUserId`, `/v1/backend/users/by-sub/:sub`, `/v1/backend/users/:appUserId/metadata`, `/v1/backend/users/:appUserId/suspend`, `/v1/backend/users/:appUserId/restore`, `/v1/backend/sessions/:id`, `/v1/backend/sessions/:id/revoke`, `/v1/backend/users/:appUserId/sessions/revoke-all`, and `/v1/backend/events`.

### Webhooks & Local DB Sync (`@secure-auth/core/webhooks`)

```ts
import {
  createWebhookRouter,
  shouldApplyVersion,
  syncUserFromClaims
} from '@secure-auth/core/webhooks';

const router = createWebhookRouter({
  secret: [process.env.AUTH_WEBHOOK_SECRET!, process.env.AUTH_WEBHOOK_SECRET_PREVIOUS!].filter(Boolean),
  toleranceSecs: 300,
  isEventProcessed: (eventId) => db.hasProcessedEvent(eventId),
  markEventProcessed: (eventId) => db.recordProcessedEvent(eventId),
  handlers: {
    'user.updated': async (event) => {
      const local = await db.findUserBySub(event.aggregate_id);
      if (shouldApplyVersion(local?.authVersion, event.aggregate_version)) {
        await db.upsertUserFromEvent(event);
      }
    }
  }
});
```

---

## 2. `@secure-auth/svelte` (Svelte 5 & SvelteKit)

### Server Hook (`src/hooks.server.ts`)

```ts
import { createAuthHandle } from '@secure-auth/svelte/server';

export const handle = createAuthHandle({
  issuerUrl: process.env.AUTH_ISSUER_URL!,
  audience: 'ferrum',
  publishableKey: process.env.AUTH_PUBLISHABLE_KEY!,
  secretKey: process.env.AUTH_SECRET_KEY,
  protectedRoutes: ['/dashboard', '/settings'],
  loginPath: '/login'
});
```

### Safe SSR Hydration (`src/routes/+layout.server.ts`)

```ts
import { serializeClientAuthState } from '@secure-auth/svelte/server';

export function load({ locals }) {
  return { auth: serializeClientAuthState(locals) };
}
```

`serializeClientAuthState` strips secret keys, raw JWTs, and refresh tokens so only public session metadata reaches the browser payload.

### OIDC Callback & Webhook Route Handlers

- `createOidcCallbackHandler({ issuerUrl, clientId, redirectUri, defaultRedirectTo })` validates PKCE state, blocks open redirects (`//evil.example`), exchanges the authorization code, and writes `HttpOnly` cookies.
- `createWebhookEndpointHandler({ secret, isEventProcessed, markEventProcessed, handlers })` validates HMAC-SHA256 headers and returns `401` on invalid or expired signatures.

### Svelte 5 Runes Components

Exported under `@secure-auth/svelte/components/*`:

- `AuthProvider.svelte`
- `SignIn.svelte` (`mode="hosted"` or `mode="direct"`)
- `SignUp.svelte` (with OTP verification step)
- `UserButton.svelte` (local or global RP-initiated sign-out)
- `SignedIn.svelte`, `SignedOut.svelte`, `ProtectedRoute.svelte`

Reactive helpers from `@secure-auth/svelte`: `createSvelteAuthContext`, `useAuth`, `useUser`, `useSession`, `usePermissions`, `canAccessRoute`.

---

## 3. `@secure-auth/react` (React 18 & 19)

```tsx
import {
  AuthProvider,
  Protect,
  SignIn,
  SignedIn,
  SignedOut,
  UserButton,
  usePermissions
} from '@secure-auth/react';
```

- Automatically binds to the installed `react` peer dependency (and exposes `createReactAuth` / `configureReactRuntime` for custom runtimes or headless testing).
- Provides `AuthProvider`, `useAuth`, `useUser`, `useSession`, `usePermissions`, `SignIn`, `SignUp`, `UserButton`, `SignedIn`, `SignedOut`, and `Protect`.

---

## 4. `@secure-auth/next` (Next.js 14/15+ App Router & Edge Runtime)

### Edge Middleware (`src/middleware.ts`)

```ts
import { authMiddleware } from '@secure-auth/next/middleware';

export const middleware = authMiddleware({
  issuerUrl: process.env.AUTH_ISSUER_URL!,
  audience: 'elysiar',
  protectedRoutes: ['/dashboard', '/billing'],
  publicRoutes: ['/', '/login', '/auth/callback', '/api/webhooks/auth'],
  loginPath: '/login'
});
```

Security guarantees:

- Executes strictly on Web Standards (`crypto.subtle` + `fetch`) with zero Node.js built-ins.
- Strips any client-supplied `x-sa-*` headers before forwarding verified identity claims (`x-sa-user-id`, `x-sa-app-user-id`, `x-sa-session-id`, `x-sa-roles`, `x-sa-permissions`, `x-sa-ver`, `x-sa-org-id`).

### App Router Server & Route Handlers (`@secure-auth/next/server`)

- `createNextAuthServer(config)` provides `auth(cookies)`, `currentUser(cookies)`, `protect(state, { permission, role })`, `createCallbackRouteHandler(config)`, and `createWebhookRouteHandler(config)`.
- `@secure-auth/next` re-exports all React client components behind a `"use client"` boundary.

---

## 5. Reference Sync Starters

- [`examples/ferrum-sveltekit`](../examples/ferrum-sveltekit/README.md) — SvelteKit 5 + `@secure-auth/svelte` starter demonstrating SSR auth, JIT projection upsert, idempotent version-guarded webhooks, and `scripts/reconcile-users.ts`.
- [`examples/elysiar-nextjs`](../examples/elysiar-nextjs/README.md) — Next.js App Router + `@secure-auth/next` starter demonstrating Edge middleware verification, OIDC callback, idempotent version-guarded webhooks, Server Component `protect()`, and `scripts/reconcile-users.ts`.

