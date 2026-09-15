# Security Architecture & Threat Mitigation

This document outlines the security controls, access models, and defensive policies implemented in `secure-auth-platform`.

---

## 1. Role-Based Access Control (RBAC)

The platform enforces fine-grained authorization via role-to-permission mapping:

### 1.1 Permission Matrix

| Permission | Description | User | Moderator | Admin |
|------------|-------------|:----:|:---------:|:-----:|
| `user:read` | Read own profile and basic user info | ✅ | ✅ | ✅ |
| `user:write` | Update own profile (name, avatar) | ✅ | ✅ | ✅ |
| `session:read` | List own active sessions | ✅ | ✅ | ✅ |
| `session:delete` | Revoke own sessions | ✅ | ✅ | ✅ |
| `user:suspend` | Suspend or flag abusive accounts | ❌ | ✅ | ✅ |
| `admin:access` | View admin dashboard & system metrics | ❌ | ❌ | ✅ |
| `user:delete` | Permanently delete user accounts | ❌ | ❌ | ✅ |
| `audit:read` | Inspect system-wide security audit logs | ❌ | ❌ | ✅ |

### 1.2 Enforcing Authorization in Axum
Permissions are resolved during JWT decoding or database extraction and enforced via Axum middleware:

```rust
// Example endpoint requiring specific permissions
Router::new()
    .route("/api/admin/users", get(list_all_users))
    .layer(from_extractor::<RequirePermission<AdminAccess>>());
```

---

## 2. Rate Limiting (Token Bucket)

Endpoints susceptible to brute force or credential stuffing attacks are strictly throttled using a Redis-backed token bucket algorithm:

| Endpoint | Window | Max Requests | Keying Strategy | Violation Action |
|----------|--------|--------------|-----------------|------------------|
| `/api/auth/login` | 60s | 5 requests | IP + Email | 429 Too Many Requests + exponential backoff |
| `/api/auth/register` | 600s | 3 registrations | IP | 429 Too Many Requests |
| `/api/auth/password-reset` | 300s | 3 requests | Email / IP | 429 Too Many Requests |
| `/api/auth/refresh` | 60s | 30 requests | Session / IP | 429 Too Many Requests |
| General API endpoints | 60s | 120 requests | User ID / IP | 429 Too Many Requests |

---

## 3. Security Headers

All HTTP responses from the backend and NGINX Ingress controller include hardened security headers:

```http
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' http://localhost:* https://*.example.com; frame-ancestors 'none';
```

---

## 4. Immutable Audit Logging

Every security-sensitive event produces an immutable audit record stored in the `audit_logs` table and forwarded to Loki:

### Logged Events
- `AUTH_LOGIN_SUCCESS` & `AUTH_LOGIN_FAILURE`
- `AUTH_LOGOUT`
- `TOKEN_ROTATED` & `TOKEN_REUSE_DETECTED`
- `PASSWORD_RESET_REQUESTED` & `PASSWORD_RESET_COMPLETED`
- `SESSION_REVOKED` & `SESSION_BULK_REVOKED`
- `USER_SUSPENDED` / `USER_DELETED`

### Audit Schema
```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(64) NOT NULL,
    ip_address INET,
    user_agent TEXT,
    status VARCHAR(16) NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## 5. Secrets Management

- **Local Development**: Managed via `.env` loaded into Docker Compose (never committed to version control).
- **Production Kubernetes**:
  - Encrypted via [External Secrets Operator (ESO)](https://external-secrets.io/) connected to AWS Secrets Manager or HashiCorp Vault.
  - Manifests in Git contain only `SecretStore` and `ExternalSecret` definitions; zero plaintext credentials exist in the Git repository.

---

## 6. Continuous Security Scanning

The platform implements automated security verification on every pull request and weekly schedule via `.github/workflows/security.yml`:

1. **Rust Audit**: `cargo audit` checks the Rust dependency graph against the RustSec Advisory Database.
2. **Node Audit**: `npm audit --workspaces` verifies all frontend and shared package dependencies.
3. **Secret Detection**: `gitleaks` scans git commits for accidentally committed secrets, API keys, or private certificates.
4. **Container Image Scanning**: `trivy` inspects Docker images for OS-level and application vulnerabilities, publishing SARIF reports directly to GitHub Security.
5. **SBOM**: Generates Software Bill of Materials in SPDX format with 90-day retention.
