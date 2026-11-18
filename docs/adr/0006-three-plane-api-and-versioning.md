---
status: accepted
---

# 0006: Three-Plane Versioned API Surface and Unified Error Envelope

We partition all HTTP routes into strictly separated, versioned planes with dedicated authentication and middleware chains, replacing legacy unversioned `/auth` and `/api/*` mounts:

1. **OIDC & Discovery (`/.well-known/*`, `/oauth2/*`)**: Standards-compliant OpenID Connect Provider + JWKS endpoints.
2. **Client / Frontend API (`/v1/client/*`)**: Browser & SDK routes identified by `pk_*` publishable keys, dynamic per-environment CORS, and `aud`-scoped asymmetric user access tokens.
3. **Backend Management & Sync API (`/v1/backend/*`)**: Server-to-server routes authenticated by scoped `sk_*` secret keys with no browser cookies or CSRF middleware.
4. **Developer Control Plane API (`/v1/control/*`)**: Dashboard routes authenticated by Developer Control Plane sessions and workspace RBAC.

All planes share a single JSON error envelope: `{ "error": { "code", "message", "request_id", "details" }, "status" }`.

## Considered Options

- **Single shared `/api/*` router with suffix-based CSRF exemptions**: Current repo pattern; mixes browser cookies, Bearer tokens, and privileged endpoints on the same middleware stack and risks accidental privilege or CSRF bypasses.
- **Strictly separated route planes with plane-specific middleware**: Ensures cookie-CSRF runs only on cookie-authenticated browser planes, dynamic CORS runs only on browser-facing planes, and `sk_*` credentials are never accepted on client or control-plane routes.

## Consequences

- Legacy `/auth`, `/api/auth`, `/users`, `/api/users`, `/sessions`, `/api/sessions` routes remain only during the Phase 00–03 transition and will be retired once `/v1/*` routes and `apps/web` cut over.
- Every error response carries an `x-request-id` correlation header and optional `error.request_id` field for support and distributed trace lookup.

