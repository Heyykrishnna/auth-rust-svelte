# Ferrum Reference Starter (SvelteKit 5 + `@secure-auth/svelte`)

Reference integration showing how a tenant application (**Ferrum**) connects to the Secure Auth Platform using `@secure-auth/svelte` and `@secure-auth/core`:

- **Stateless SSR Auth & Transparent Refresh** (`src/hooks.server.ts`): Validates ES256 access tokens against the auth server JWKS, silently rotates expired sessions using the `HttpOnly` refresh cookie, protects `/dashboard` and `/settings`, and performs JIT user projection upserts on every verified request.
- **Zero-Secret Client Hydration** (`src/routes/+layout.server.ts`): Serializes only safe public claims via `serializeClientAuthState(locals)`.
- **OIDC PKCE `S256` Callback** (`src/routes/auth/callback/+server.ts`): Validates `state` and PKCE `code_verifier`, exchanges the code at `/oauth2/token`, and sets `HttpOnly` session cookies.
- **Idempotent, Version-Guarded Webhook Sync** (`src/routes/api/webhooks/auth/+server.ts`): Verifies `X-Auth-Signature` with dual-secret rotation support, deduplicates by `X-Auth-Event-Id`, and protects local user rows against out-of-order deliveries with `shouldApplyVersion`.
- **Cursor Reconciliation** (`scripts/reconcile-users.ts`): Streams updated users from `GET /v1/backend/users?updated_since=...` via `iterateUsersUpdatedSince`.

## Configuration

Copy `.env.example` to `.env` and populate the values provisioned for your Ferrum tenant app in the Admin Console.

