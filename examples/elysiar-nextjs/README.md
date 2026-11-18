# Elysiar Reference Starter (Next.js 15+ App Router + `@secure-auth/next`)

Reference integration showing how a tenant application (**Elysiar**) connects to the Secure Auth Platform using `@secure-auth/next`, `@secure-auth/react`, and `@secure-auth/core`:

- **Edge Middleware** (`src/middleware.ts`): Runs on Web Standards (`crypto.subtle` + `fetch`, zero Node.js built-ins), verifies ES256 access tokens via cached JWKS, strips spoofed `x-sa-*` headers, and redirects unauthenticated visitors on `/dashboard/*` and `/billing/*` to `/login`.
- **App Router OIDC Callback** (`src/app/auth/callback/route.ts`): Validates PKCE `S256` state and exchanges the authorization code at `/oauth2/token`, storing `HttpOnly` session cookies.
- **App Router Webhook Sync** (`src/app/api/webhooks/auth/route.ts`): Verifies `X-Auth-Signature` with dual-secret rotation support, deduplicates by `X-Auth-Event-Id`, and guards local DB projections against out-of-order updates via `shouldApplyVersion`.
- **Server Component Loader** (`src/app/dashboard/page.ts`): Uses `createNextAuthServer`, `auth()`, `protect()`, and JIT claim projection upsert (`syncUserFromClaims`).
- **Cursor Reconciliation** (`scripts/reconcile-users.ts`): Streams updated users from `GET /v1/backend/users?updated_since=...` via `iterateUsersUpdatedSince`.

## Configuration

Copy `.env.example` to `.env.local` and populate the values provisioned for your Elysiar tenant app in the Admin Console.

