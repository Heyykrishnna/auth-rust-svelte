# OpenID Connect (OIDC) Provider & Centralized SSO Integration Guide

This specification documents how consuming applications (**Ferrum**, **Elysiar**, or any standard OIDC Relying Party) authenticate users through the Central Identity Provider with first-party SSO sessions, mandatory PKCE (`S256`), and strict tenant isolation.

---

## 1. Discovery & Endpoints

Standard OIDC clients can auto-configure using:

```http
GET /.well-known/openid-configuration
```

| Endpoint | Path | Notes |
| --- | --- | --- |
| Discovery | `GET /.well-known/openid-configuration` | Cached `public, max-age=300` |
| Public JWKS | `GET /.well-known/jwks.json` | Active & retiring `ES256` keys with `ETag` / `304` |
| Authorization | `GET /oauth2/authorize` | Authorization Code + mandatory PKCE `S256` |
| Token | `POST /oauth2/token` | `authorization_code` & `refresh_token` grants (form or JSON), `Cache-Control: no-store` |
| UserInfo | `GET /oauth2/userinfo` | Requires Bearer `access_token` with `openid` scope |
| Revocation | `POST /oauth2/revoke` | RFC 7009 local refresh-token & `app_session` revocation |
| Logout | `GET`/`POST /oauth2/logout` | RP-Initiated local (`global=false`) or global (`global=true`) logout |

---

## 2. Authorization Code + PKCE Flow (`S256`)

### Step 1: Redirect Browser to `/oauth2/authorize`

```http
GET /oauth2/authorize
  ?client_id=<environment_id_or_pk>
  &redirect_uri=https://app.ferrum.example/auth/callback
  &response_type=code
  &scope=openid%20profile%20email%20offline_access
  &state=<random_csrf_state>
  &nonce=<random_id_token_nonce>
  &code_challenge=<BASE64URL(SHA256(code_verifier))>
  &code_challenge_method=S256
```

**Strict Validation Rules:**
- `client_id` maps to a single `application_environment` (accepts either the `environment_id` UUID or an active publishable key `pk_test_*` / `pk_live_*`).
- `redirect_uri` is verified by **exact string match** against `application_domains.redirect_uris`. Invalid `client_id` or `redirect_uri` returns an error response directly on the IdP and **never** issues a `302` redirect (preventing open redirects).
- `code_challenge_method=S256` is mandatory; `plain` and missing PKCE challenges are rejected.

### Step 2: Central SSO Policy Evaluation

The IdP inspects its first-party `HttpOnly`, `SameSite=Lax` cookie (`__Host-idp_sso_session` over HTTPS; `idp_sso_session` in local HTTP dev):

1. No valid `sso_session` → redirects to `/portal/login?tx=<id>` (or redirects back with `error=login_required` when `prompt=none`).
2. `prompt=login` or `max_age` exceeded (including `max_age=0`) → forces interactive re-authentication.
3. Suspended global `users.status != 'active'` or suspended `application_users.status != 'active'` → redirects back with `error=access_denied`.
4. Existing active membership for target environment → immediately issues a 60-second single-use authorization code (**zero password prompt**).
5. First-time visit to a new application (e.g., Ferrum user opening Elysiar):
   - `auto_join` → atomically creates `application_users` + default role + `membership.created` outbox event and issues authorization code immediately.
   - `consent_required` → routes to `/portal/consent?tx=<id>` (or returns `error=consent_required` when `prompt=none`).
   - `invite_only` → redirects back with `error=access_denied`.

### Step 3: Exchange Code at `/oauth2/token`

```http
POST /oauth2/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code
&client_id=<environment_id_or_pk>
&code=<authorization_code>
&redirect_uri=https://app.ferrum.example/auth/callback
&code_verifier=<original_pkce_verifier>
```

**Security Guarantees:**
- Authorization codes expire in **60 seconds**, are 256-bit random, stored SHA-256 hashed, and strictly single-use (`SELECT ... FOR UPDATE`).
- **Code Replay Protection (RFC 9700):** Presenting an already-consumed code immediately revokes the `app_session` and `refresh_tokens` family issued from that code, emits a `session.revoked` outbox event, and logs `OIDC_CODE_REPLAY_DETECTED`.
- Issued `access_token` (`typ: "at+jwt"`) and `id_token` (`typ: "JWT"`) are signed with `ES256`, carry `aud = environment_id`, and preserve `nonce`, `auth_time`, `amr`, `acr`, `app_user_id`, and `ver`.

---

## 3. Hosted Auth & Consent Portal (`apps/web/src/routes/portal/*`)

Hosted end-user pages live strictly under `/portal/*` (separated from the Developer Dashboard `/control/*`) and serve:
- `Content-Security-Policy: frame-ancestors 'none'`
- `X-Frame-Options: DENY`
- `Referrer-Policy: no-referrer`
- Transaction-bound CSRF tokens validated on every `/oauth2/portal/transactions/:tx_id/*` submission.

Routes:
- `/portal/login?tx=<id>` — Branded sign-in page
- `/portal/sign-up?tx=<id>` — Branded registration + 6-digit OTP verification + global email collision handoff
- `/portal/consent?tx=<id>` — First-time app-join consent screen showing requested OIDC scopes
- `/portal/logout` — Central SSO account switcher, active `sso_sessions` manager, and local/global sign-out

---

## 4. Local vs. Global Logout

| Mode | Call | Effect |
| --- | --- | --- |
| **Local Logout** | `POST /oauth2/revoke` or `GET /oauth2/logout?id_token_hint=...&global=false` | Revokes only the requesting application's `app_session` and refresh family. Central `sso_session` and other apps remain active. |
| **Global Logout** | `GET /oauth2/logout?id_token_hint=...&post_logout_redirect_uri=...&global=true` | Revokes the IdP `sso_session` and **all** linked `app_sessions` across every environment, clears the IdP SSO cookie, and emits `session.revoked` outbox events to every affected environment. |

