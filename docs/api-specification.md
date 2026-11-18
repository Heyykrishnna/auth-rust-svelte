# API & Event Contract Specification (`v1`)

This specification establishes the contract across all three platform planes plus the canonical webhook event envelope before multi-tenant implementation begins in Phase 01.

---

## 1. Global Conventions

### Unified Error Envelope
Every non-2xx HTTP response across all planes uses the following JSON shape:

```json
{
  "error": {
    "code": "INVALID_TOKEN",
    "message": "Token signature verification failed",
    "request_id": "9a7d8a32-4c2f-4e4e-95e2-f3bc08f23210",
    "details": null
  },
  "status": 401
}
```

- `error.code`: Stable machine-readable `SCREAMING_SNAKE_CASE` code (`VALIDATION_ERROR`, `UNAUTHORIZED`, `FORBIDDEN`, `INVALID_CREDENTIALS`, `INVALID_TOKEN`, `TOKEN_EXPIRED`, `SESSION_NOT_FOUND`, `USER_NOT_FOUND`, `EMAIL_ALREADY_EXISTS`, `CSRF_TOKEN_INVALID`, `TOO_MANY_REQUESTS`, `OIDC_ERROR`, `DATABASE_ERROR`, `CACHE_ERROR`, `INTERNAL_ERROR`).
- `error.message`: Safe user-facing summary (never includes stack traces, SQL errors, or secret material).
- `error.request_id`: Correlation ID matching the `x-request-id` response header when available.
- `error.details`: Optional structured validation/field metadata (omitted when `null`).

### Pagination Convention
- Operational/sync endpoints use **cursor-based keyset pagination**: `?cursor=<opaque>&limit=100` (default `50`, max `100`), returning `{ "data": [...], "next_cursor": "..." | null, "has_more": true | false }`.

---

## 2. Plane 1 — OIDC Provider & Discovery

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| `GET` | `/.well-known/openid-configuration` | Public | OIDC discovery document |
| `GET` | `/.well-known/jwks.json` | Public | Active & retiring public signing keys (`kid`, `RS256`/`ES256`) |
| `GET` | `/oauth2/authorize` | IdP `__Host-idp_sso_session` cookie (optional) | Authorization Code + PKCE (`S256`) entry point & SSO policy check |
| `POST` | `/oauth2/token` | PKCE `code_verifier` (+ optional client secret) | Code-to-token exchange & `refresh_token` rotation |
| `GET` | `/oauth2/userinfo` | Bearer `access_token` (`openid` scope) | Authoritative profile claims for authenticated user |
| `POST` | `/oauth2/revoke` | Client auth / refresh token | RFC 7009 token & `app_session` revocation |
| `GET`/`POST` | `/oauth2/logout` | `id_token_hint` / IdP cookie | Local or global RP-Initiated Logout |

### Standard Asymmetric Access Token Claims (`typ: "at+jwt"`)
```json
{
  "iss": "https://auth.platform.example",
  "sub": "usr_01J...",
  "aud": "env_01J...",
  "azp": "pk_live_...",
  "app_user_id": "app_usr_01J...",
  "sid": "sess_01J...",
  "org_id": "org_01J...",
  "org_role": "org:admin",
  "roles": ["user"],
  "permissions": ["profile.read", "profile.write"],
  "ver": 4,
  "iat": 1763420000,
  "exp": 1763420300,
  "jti": "jti_01J..."
}
```

---

## 3. Plane 2 — Client / Frontend API (`/v1/client/*`)

Requires `X-Publishable-Key: pk_test_* | pk_live_*` (or environment-bound host resolution) and dynamic per-environment CORS validation.

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| `POST` | `/v1/client/auth/sign-up` | `pk_*` | Initiate registration (sends verification OTP) |
| `POST` | `/v1/client/auth/sign-up/verify` | `pk_*` | Verify OTP, create identity/membership/session |
| `POST` | `/v1/client/auth/sign-up/resend-otp` | `pk_*` | Rate-limited OTP resend |
| `POST` | `/v1/client/auth/sign-in` | `pk_*` | Email/password sign-in (app-local or Hosted Portal) |
| `POST` | `/v1/client/auth/refresh` | `pk_*` + refresh token | Rotate refresh token & mint fresh access JWT |
| `POST` | `/v1/client/auth/sign-out` | `pk_*` + access/refresh | Revoke current `app_session` |
| `POST` | `/v1/client/auth/password/forgot` | `pk_*` | Initiate password reset |
| `POST` | `/v1/client/auth/password/reset` | `pk_*` | Consume reset token & revoke all sessions |
| `POST` | `/v1/client/auth/password/change` | Bearer access JWT | Change password for authenticated user |
| `GET` | `/v1/client/auth/oauth/:provider/start` | `pk_*` | Start upstream Google/GitHub PKCE OAuth flow |
| `POST` | `/v1/client/auth/oauth/:provider/callback` | `pk_*` | Complete upstream OAuth callback |
| `GET`/`PATCH` | `/v1/client/users/me` | Bearer access JWT | Read/update current user profile |
| `GET`/`DELETE` | `/v1/client/sessions` / `/:id` | Bearer access JWT | Inspect & revoke environment `app_sessions` |
| `POST` | `/v1/client/sessions/active-organization` | Bearer access JWT | Switch active org & mint org-scoped access JWT |
| `GET`/`POST` | `/v1/client/organizations` | Bearer access JWT | List/create organizations (Phase 08) |

---

## 4. Plane 3 — Backend Sync & Management API (`/v1/backend/*`)

Authenticated strictly via `Authorization: Bearer sk_test_* | sk_live_*` with scope checks. Zero cookies, zero CSRF middleware.

| Method | Path | Required Scope | Purpose |
| --- | --- | --- | --- |
| `GET` | `/v1/backend/users` | `sync:read` or `users:read` | Cursor-paginated sync (`?updated_since=...&cursor=...&limit=100`) |
| `GET` | `/v1/backend/users/:app_user_id` | `users:read` | Fetch membership + global identity summary |
| `GET` | `/v1/backend/users/by-sub/:sub` | `users:read` | Lookup environment membership by immutable global `sub` |
| `PATCH` | `/v1/backend/users/:app_user_id/metadata` | `users:write` | Update `external_id`, `public_metadata`, `private_metadata` |
| `POST` | `/v1/backend/users/:app_user_id/suspend` | `users:write` | Suspend user within this environment & revoke `app_sessions` |
| `POST` | `/v1/backend/users/:app_user_id/restore` | `users:write` | Restore environment membership |
| `GET` | `/v1/backend/sessions/:session_id` | `sessions:read` | Authoritative session status check |
| `POST` | `/v1/backend/sessions/:session_id/revoke` | `sessions:revoke` | Force-revoke a single `app_session` |
| `POST` | `/v1/backend/users/:app_user_id/sessions/revoke-all` | `sessions:revoke` | Revoke all `app_sessions` for user in this environment |
| `GET` | `/v1/backend/events` | `sync:read` | Catch-up event log (`?after_event_id=...&types=...&limit=100`) |
| `GET`/`POST` | `/v1/backend/organizations` | `orgs:read` / `orgs:manage` | Sync & manage environment organizations (Phase 08) |

---

## 5. Plane 4 — Developer Control Plane API (`/v1/control/*`)

Authenticated via Developer session + Workspace RBAC (`owner`, `admin`, `developer`, `viewer`) + double-submit CSRF protection.

- `/v1/control/auth/*` — Developer sign-up, sign-in, sign-out, session management
- `/v1/control/workspaces/*` — Workspaces & team member role management
- `/v1/control/applications` & `/:app_id` — Application CRUD & suspension
- `/v1/control/applications/:app_id/environments/:env_kind` — Environment settings, enrollment policy, branding
- `/v1/control/applications/:app_id/environments/:env_kind/credentials` — Publishable & secret key creation/rotation/revocation
- `/v1/control/applications/:app_id/environments/:env_kind/domains` — Allowed origins, redirect URIs, DNS verification
- `/v1/control/applications/:app_id/environments/:env_kind/oauth-providers` — Social provider settings
- `/v1/control/applications/:app_id/environments/:env_kind/webhooks` — Webhook endpoints, test ping, delivery logs, manual replay
- `/v1/control/applications/:app_id/environments/:env_kind/users` — End-user directory, roles, sessions, suspension
- `/v1/control/applications/:app_id/environments/:env_kind/roles` — Custom roles & permissions (Phase 08)
- `/v1/control/applications/:app_id/environments/:env_kind/audit-logs` & `/analytics` — Audit explorer & rollups

---

## 6. Canonical Outbound Webhook Envelope (`v1`)

### HTTP Headers
- `Content-Type: application/json`
- `X-Auth-Event-Id: evt_01J...`
- `X-Auth-Timestamp: 1763420000`
- `X-Auth-Signature: v1=<hex_hmac_sha256(timestamp + "." + raw_body)>` (comma-separated multiple `v1=` signatures during secret rotation grace window)

### Payload Structure
```json
{
  "spec_version": "1.0",
  "event_id": "evt_01J...",
  "event_type": "user.updated",
  "environment_id": "env_01J...",
  "occurred_at": "2026-11-18T00:00:00Z",
  "aggregate_type": "application_user",
  "aggregate_id": "app_usr_01J...",
  "aggregate_version": 4,
  "data": {}
}
```

