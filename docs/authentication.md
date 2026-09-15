# Authentication & Identity Architecture

This document specifies the authentication flows, token rotation mechanics, session management, and cryptographic standards implemented across `secure-auth-platform`.

---

## 1. Credentials & Cryptography

### 1.1 Password Hashing (Argon2id)
All user passwords submitted during registration or password change are hashed using **Argon2id**, the winner of the Password Hashing Competition (PHC), configured with parameters optimized for resistance against GPU/ASIC attacks:
- **Variant**: `Argon2id`
- **Memory Cost ($m$)**: 64 MB (65536 KiB)
- **Time Cost ($t$)**: 3 iterations
- **Parallelism ($p$)**: 4 threads
- **Salt**: 16 bytes of cryptographically secure random bytes (`OsRng`)

Plaintext passwords are never logged, never returned in API payloads, and never stored in memory beyond the immediate verification scope.

---

## 2. Token Lifecycle & Rotation

The platform issues a dual-token pair upon successful authentication:

| Token Type | Lifespan | Transmission | Purpose |
|------------|----------|--------------|---------|
| **Access Token** | 15 minutes | HTTP Authorization header (`Bearer <JWT>`) | Stateless API request authorization |
| **Refresh Token** | 7 days | `HttpOnly`, `Secure`, `SameSite=Strict` Cookie | Issuing new access tokens via rotation |

```mermaid
sequenceDiagram
    autonumber
    actor User as Client (Web / Browser)
    participant API as Auth API (Axum)
    participant Redis as Redis Cache
    participant DB as PostgreSQL

    Note over User,DB: User Login & Token Generation
    User->>API: POST /api/auth/login (email, password)
    API->>DB: Fetch user by email
    API->>API: Verify Argon2id hash
    API->>DB: Create session & token family record
    API->>Redis: Cache session & family state
    API-->>User: Set-Cookie: refresh_token (HttpOnly, Secure)<br/>Return: access_token (JWT, 15m)

    Note over User,DB: Access Token Refresh with Reuse Detection
    User->>API: POST /api/auth/refresh (Cookie: refresh_token)
    API->>Redis: Check token family counter & hash
    alt Token Hash Matches (Valid Rotation)
        API->>API: Generate new Access Token (15m) & Refresh Token (7d)
        API->>Redis: Update token family (advance counter, store new hash)
        API->>DB: Persist new refresh token hash
        API-->>User: Set-Cookie: new_refresh_token<br/>Return: new_access_token
    else Token Hash Already Used (Compromised / Replay Attack)
        API->>Redis: Invalidate token family & delete session
        API->>DB: Revoke all refresh tokens in family
        API->>DB: Record security audit log: REUSE_DETECTED
        API-->>User: 401 Unauthorized (Session Revoked)
    end
```

### 2.1 Refresh Token Rotation & Reuse Detection
To eliminate the risk of long-lived token theft, every refresh request issues a brand-new refresh token and invalidates the previous one:
1. **Token Families**: Every login creates a unique `family_id`.
2. **Replay Detection**: If a client attempts to exchange an already-used refresh token, the server assumes token theft has occurred.
3. **Immediate Revocation**: The server immediately revokes the **entire token family** and deletes the underlying active session, forcing all legitimate and malicious holders to re-authenticate.

---

## 3. OIDC / Social Authentication

The platform supports OpenID Connect (OIDC) and OAuth 2.0 with:
- **Google Identity Services**
- **GitHub OAuth**

### OIDC Flow
```mermaid
sequenceDiagram
    autonumber
    actor User as User Browser
    participant API as Auth API
    participant IdP as External IdP (Google / GitHub)
    participant DB as PostgreSQL

    User->>API: GET /api/auth/oidc/:provider (redirect)
    API->>API: Generate cryptographically secure state & PKCE code_verifier
    API->>User: 302 Redirect to IdP Auth URL
    User->>IdP: Authenticate & Authorize Scopes
    IdP-->>User: 302 Redirect to /auth/callback?code=...&state=...
    User->>API: GET /api/auth/oidc/:provider/callback?code=...&state=...
    API->>API: Validate state parameter (CSRF protection)
    API->>IdP: POST /token (exchange code + code_verifier)
    IdP-->>API: Returns id_token & access_token
    API->>IdP: Fetch user profile (email, verified, avatar)
    API->>DB: Link or create user & oidc_accounts record
    API-->>User: Establish session & redirect to /dashboard
```

---

## 4. Session Management

- **Concurrent Multi-Device Sessions**: Users can maintain active sessions across multiple browsers or devices simultaneously.
- **Session Metadata**: Every session captures:
  - Client IP Address
  - `User-Agent` string (browser and OS details)
  - Creation timestamp and `last_used_at` timestamp
- **Session Termination**:
  - `POST /api/sessions/:id/revoke`: Terminate a specific session.
  - `POST /api/sessions/revoke-others`: Terminate all sessions except the current active session.
  - `POST /api/auth/logout`: Revoke current session and clear authentication cookies.

---

## 5. Cookie Security & CSRF Defense

1. **Cookie Configuration**:
   - `HttpOnly`: Prevents JavaScript access (`document.cookie`), mitigating XSS-based token theft.
   - `Secure`: Ensures cookies are transmitted solely over TLS (HTTPS).
   - `SameSite=Strict`: Restricts cookies from being transmitted in cross-site requests.
2. **CSRF Double-Submit Mechanism**:
   - For state-mutating requests (`POST`, `PUT`, `DELETE`, `PATCH`), clients transmit an anti-CSRF token in the `X-CSRF-Token` header matching a cryptographically signed cookie token.
