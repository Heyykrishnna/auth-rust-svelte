# Security Threat Model

This threat model identifies the primary security risks for the multi-tenant Identity Infrastructure Platform, mapping each threat to concrete architectural controls and the phase where automated verification is enforced.

---

## 1. Cross-Tenant & Cross-Environment Data Leakage

| Threat | Attack Vector | Mitigations | Verification Phase |
| --- | --- | --- | --- |
| **SQL tenant escape** | Missing `WHERE environment_id = $1` or connection-pool RLS bleed | Composite foreign keys `(environment_id, ...)` + transaction-scoped `SET LOCAL app.current_environment_id` via `with_tenant_tx` + separate global vs. tenant repository modules | Phase 01, Phase 10 |
| **Redis cache collision** | Overlapping session/OTP/rate-limit keys across apps | Mandatory key namespacing (`env:{environment_id}:...` vs. `idp:global:...`) | Phase 01, Phase 03 |
| **JWT audience confusion** | Replaying a valid Ferrum access token against Elysiar API | Strict `aud == expected_environment_id` and `iss` validation in both `auth-api` and `@secure-auth/core/server`; asymmetric `ES256`/`RS256` with `typ: "at+jwt"` | Phase 03, Phase 04, Phase 06 |
| **Global event over-broadcast** | Global `users` update sent to an app the user never joined | Outbox fan-out joins strictly against `application_users` memberships for that `user_id` | Phase 05, Phase 10 |
| **Dev/Prod crossover** | Using `sk_test_*` or `pk_test_*` against production resources | Prefix-to-environment validation + distinct `environment_id` rows for `development` and `production` | Phase 02, Phase 10 |

---

## 2. OAuth2 / OIDC & SSO Protocol Attacks

| Threat | Attack Vector | Mitigations | Verification Phase |
| --- | --- | --- | --- |
| **Authorization code interception** | Malicious app intercepts `code` on redirect | Mandatory PKCE (`S256` only; reject `plain`), <= 60s code TTL, hashed single-use code storage | Phase 04, Phase 10 |
| **Authorization code replay** | Re-submitting a consumed `code` to `/oauth2/token` | Atomic code consumption; replay immediately revokes all tokens and the `app_session` minted from that code | Phase 04, Phase 10 |
| **Open redirect / phishing** | Crafted `redirect_uri` or `post_logout_redirect_uri` | Exact-string matching against registered `application_domains` (HTTPS required in prod, no wildcards); never redirect on invalid `client_id` or `redirect_uri` | Phase 02, Phase 04, Phase 10 |
| **Login CSRF / State fixation** | Forcing victim browser to complete attacker's OAuth callback | Persisted single-use `state` and `nonce` bound to `authentication_transactions` + `id_token.nonce` verification | Phase 03, Phase 04 |
| **Clickjacking on Hosted Consent** | Embedding Hosted Auth/Consent Portal in invisible iframe | `Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY` on all Hosted Portal responses | Phase 04, Phase 10 |
| **Silent SSO privacy leak** | Unfamiliar app harvesting user profile without consent | Default `enrollment_policy = consent_required`; `prompt=none` returns `consent_required` rather than auto-enrolling | Phase 04 |

---

## 3. Credential, Key & Session Compromise

| Threat | Attack Vector | Mitigations | Verification Phase |
| --- | --- | --- | --- |
| **Secret key (`sk_*`) leakage** | DB read compromise or log exposure | High-entropy CSPRNG generation, one-time plaintext reveal, cryptographic hash storage, constant-time compare, scoped permissions, log redaction | Phase 02, Phase 09 |
| **Signing key compromise** | DB dump exposing JWT signing private key | Envelope encryption (AES-256-GCM + KMS/KEK `kek_version`) on `signing_keys.private_key_encrypted`; immediate key revocation + zero-downtime `kid` rotation runbook | Phase 03, Phase 10, Phase 11 |
| **Refresh token theft & replay** | Stolen refresh token used after legitimate rotation | Opaque hashed refresh tokens with RFC 6819 `family_id` tracking inside `SELECT ... FOR UPDATE` transaction; reuse revokes entire family + `app_session` | Phase 03, Phase 10 |
| **JWT algorithm confusion** | `alg: none` or HS256-signed with public key | Explicit allowlist of asymmetric algorithms (`RS256` / `ES256`) during decode; zero HS256 paths in Phase 03+ | Phase 03, Phase 10 |
| **Missing-claim permission bypass** | Token with empty claims receiving fallback permissions | Fail-closed auth middleware (remove legacy default-permission fallback in `middleware/auth.rs`) | Phase 03 |

---

## 4. Account Takeover & Abuse

| Threat | Attack Vector | Mitigations | Verification Phase |
| --- | --- | --- | --- |
| **Pre-hijacking via unverified OAuth email** | Attacker registers unverified email on provider and links victim's account | Auto-link by email **only** when both upstream `email_verified == true` and local `user_emails.verified_at IS NOT NULL` | Phase 03, Phase 10 |
| **Global email collision overwrite** | Attacker signs up on App B with victim's App A email to overwrite password | Never overwrite global password on sign-up collision; require authentication of existing global identity | Phase 03 |
| **Credential stuffing & OTP brute force** | Automated password/OTP guessing | Argon2id on bounded blocking pool (prevents event-loop DoS), Redis per-IP + per-email lockouts, max 5 OTP attempts, constant-time comparison | Phase 03, Phase 10 |
| **Organization invitation replay / hijacking** | Reusing accepted link or accepting with wrong email | Single-use hashed `token_hash`, 7-day expiry, verified-email match check, atomic `accepted` transition | Phase 08, Phase 10 |

---

## 5. Webhook Infrastructure & Sync Attacks

| Threat | Attack Vector | Mitigations | Verification Phase |
| --- | --- | --- | --- |
| **SSRF & DNS rebinding** | Developer registers webhook resolving to `169.254.169.254`, `127.0.0.1`, or internal K8s service | Connection-time resolved IP validation blocking loopback/RFC1918/link-local/CGNAT/IPv6-mapped ranges, `redirect::Policy::none()`, strict timeouts, plus Kubernetes egress `NetworkPolicy` | Phase 05, Phase 10, Phase 11 |
| **Forged / replayed webhooks** | Attacker posts fake `user.updated` or replays stale event to consuming service | Mandatory HMAC-SHA256 signature over `timestamp + "." + raw_body`, <= 5 min timestamp skew check, `X-Auth-Event-Id` dedup, monotonic `aggregate_version` guard | Phase 05, Phase 06, Phase 10 |
| **Slow-loris / memory exhaustion on worker** | Malicious webhook endpoint trickles response or sends 100 MB body | Strict 3s connect / 10s total timeout, 4 KB response excerpt cap, circuit breaker auto-pausing failing endpoints | Phase 05, Phase 10 |

