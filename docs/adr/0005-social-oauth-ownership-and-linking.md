---
status: accepted
---

# 0005: Platform-Default Social OAuth with Optional Per-Environment Credentials and Verified-Only Linking

Global `oauth_accounts` link external provider identities (`TEXT` provider keys rather than Postgres ENUMs) to `users`, using platform-managed Google/GitHub OAuth credentials by default with an optional per-environment override (`application_oauth_providers`, envelope-encrypted at rest) for branded consent screens. Automatic linking to an existing global `User` by email is permitted **only** when both the upstream provider asserts `email_verified == true` and the local `user_emails` address is verified.

## Considered Options

- **Global IdP social credentials only**: Simplest callback routing and unified Google/GitHub consent, but prevents production apps from displaying their own brand on Google/GitHub OAuth screens.
- **Per-app OAuth accounts only**: Forces users to re-link Google/GitHub independently per application and breaks global identity continuity.
- **Global `oauth_accounts` + optional per-environment `application_oauth_providers` + verified-only linking**: Preserves global identity portability, enables custom branded OAuth apps when needed, and prevents pre-hijacking account takeover via unverified emails.

## Consequences

- Upstream social login must enforce PKCE (`S256`), single-use persisted `state`, and `nonce` verification (for OIDC providers like Google).
- If an upstream provider returns an unverified email matching an existing user, linking is blocked until the user authenticates to the existing account and explicitly links the provider.
- Provider identifiers migrate from the legacy `oidc_provider` PostgreSQL ENUM to validated `TEXT` so future providers require no DDL enum locks.

