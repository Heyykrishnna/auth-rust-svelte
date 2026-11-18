# Authentication, Centralized SSO & Synchronization Sequences

This document defines the canonical runtime interaction flows for cross-application OIDC SSO, refresh token rotation, local vs. global logout, and 4-layer consuming service synchronization.

---

## 1. Initial Sign-In on Ferrum (Application A) + Cross-App SSO into Elysiar (Application B)

```mermaid
sequenceDiagram
    actor User
    participant Ferrum as Ferrum (App A)
    participant IdP as Central IdP (Hosted Portal + auth-api)
    participant Elysiar as Elysiar (App B)

    User->>Ferrum: Opens Ferrum
    Ferrum->>IdP: GET /oauth2/authorize?client_id=ferrum&redirect_uri=...&scope=openid+email+profile&state=S1&nonce=N1&code_challenge=C1&code_challenge_method=S256
    IdP->>IdP: Validates client_id & exact redirect_uri, stores authentication_transaction
    IdP->>User: Renders Hosted Sign-In (branded for Ferrum)
    User->>IdP: Submits credentials / OTP / Social OAuth
    IdP->>IdP: TX: verifies user, ensures application_users(ferrum), creates sso_session, emits outbox_event
    IdP-->>User: Sets first-party __Host-idp_sso_session cookie (HttpOnly, Secure, SameSite=Lax)
    IdP->>Ferrum: 302 redirect_uri?code=CODE_A&state=S1
    Ferrum->>IdP: POST /oauth2/token (code=CODE_A, code_verifier=V1, client_id=ferrum)
    IdP->>IdP: TX: consumes CODE_A, creates app_session(ferrum) + refresh_token family
    IdP-->>Ferrum: Ferrum access_token (aud=ferrum), id_token (aud=ferrum, nonce=N1), refresh_token

    User->>Elysiar: Opens Elysiar
    Elysiar->>IdP: GET /oauth2/authorize?client_id=elysiar&redirect_uri=...&scope=openid+email+profile&state=S2&nonce=N2&code_challenge=C2&code_challenge_method=S256
    IdP->>IdP: Validates __Host-idp_sso_session + policy (max_age, prompt, user.status, app_user.status, enrollment_policy)
    alt enrollment_policy == consent_required & first visit to Elysiar
        IdP->>User: Shows "Continue to Elysiar as alice@example.com?"
        User->>IdP: Approves consent
    end
    IdP->>IdP: TX: ensures application_users(elysiar), emits membership.created, issues CODE_B
    IdP->>Elysiar: 302 redirect_uri?code=CODE_B&state=S2
    Elysiar->>IdP: POST /oauth2/token (code=CODE_B, code_verifier=V2, client_id=elysiar)
    IdP->>IdP: TX: consumes CODE_B, creates app_session(elysiar) + refresh_token family
    IdP-->>Elysiar: Elysiar access_token (aud=elysiar), id_token (aud=elysiar, nonce=N2), refresh_token
```

---

## 2. Refresh Token Rotation & Reuse Detection (RFC 6819)

```mermaid
sequenceDiagram
    participant App as Consuming App / SDK
    participant API as auth-api
    participant DB as PostgreSQL

    App->>API: POST /oauth2/token (grant_type=refresh_token, refresh_token=RT_1)
    API->>DB: BEGIN TX + SELECT refresh_token WHERE token_hash = sha256(RT_1) FOR UPDATE
    alt RT_1 is active and matches app_session
        API->>DB: Mark RT_1 revoked_at = NOW()
        API->>DB: Verify users.status & application_users.status == active, resolve fresh roles/permissions
        API->>DB: Insert RT_2 in same family_id, update app_session.last_used_at
        API->>DB: COMMIT TX
        API-->>App: New short-lived asymmetric access_token + RT_2
    else RT_1 already revoked (token theft / replay)
        API->>DB: Revoke entire family_id & linked app_session
        API->>DB: Insert high-severity REFRESH_TOKEN_REUSE_DETECTED audit_log & session.revoked outbox_event
        API->>DB: COMMIT TX
        API-->>App: 401 INVALID_TOKEN (Refresh token reuse detected)
    end
```

---

## 3. Local Logout vs. Global SSO Logout

```mermaid
sequenceDiagram
    actor User
    participant Ferrum as Ferrum (App A)
    participant IdP as Central IdP
    participant Worker as webhook_worker
    participant Elysiar as Elysiar (App B)

    alt Local Logout (Ferrum only)
        User->>Ferrum: Clicks Sign Out
        Ferrum->>IdP: POST /oauth2/revoke (Ferrum refresh_token)
        IdP->>IdP: Revokes Ferrum app_session + family; keeps sso_session & Elysiar app_session active
        Ferrum-->>User: Clears Ferrum session
    else Global Logout (All Apps)
        User->>Ferrum: Clicks Sign Out Everywhere
        Ferrum->>IdP: GET /oauth2/logout?id_token_hint=...&post_logout_redirect_uri=...&global=true
        IdP->>IdP: TX: Revokes sso_session + all linked app_sessions (Ferrum & Elysiar), emits session.revoked outbox_events
        IdP-->>User: Clears __Host-idp_sso_session cookie & redirects to post_logout_redirect_uri
        Worker->>Ferrum: HMAC-signed webhook session.revoked
        Worker->>Elysiar: HMAC-signed webhook session.revoked
    end
```

---

## 4. Four-Layer Consuming Service Synchronization & Downtime Recovery

```mermaid
sequenceDiagram
    participant IdP as auth-api
    participant DB as Platform Postgres
    participant Worker as webhook_worker
    participant Service as Consuming Service (Ferrum / Elysiar)

    Note over IdP,Service: Layer 1 — Push Sync (Real-Time)
    IdP->>DB: TX: UPDATE users / application_users (version = version + 1) + INSERT outbox_events
    Worker->>DB: SELECT ... FROM outbox_events FOR UPDATE SKIP LOCKED
    Worker->>Service: POST /api/webhooks/auth (X-Auth-Event-Id, X-Auth-Timestamp, X-Auth-Signature)
    Service->>Service: Verify HMAC-SHA256 + timestamp skew + idempotent upsert WHERE incoming.version >= local.version
    Service-->>Worker: 200 OK

    Note over IdP,Service: Layer 2 — JIT Claim Sync (Zero First-Login Race)
    Service->>Service: Verifies asymmetric JWT via cached JWKS -> JIT-upserts local user by sub/app_user_id

    Note over IdP,Service: Layer 3 — Pull Reconciliation (After Consumer Downtime)
    Service->>IdP: GET /v1/backend/users?updated_since=2026-11-18T00:00:00Z&cursor=... (Bearer sk_live_*)
    IdP-->>Service: Paginated environment-scoped users + next_cursor
    Service->>Service: Idempotent version-guarded upsert until next_cursor == null
```

