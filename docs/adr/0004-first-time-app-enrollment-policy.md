---
status: accepted
---

# 0004: Configurable First-Time App Enrollment Policy for SSO

When a global `User` with a valid central `sso_session` initiates OIDC login to an `Environment` where they do not yet hold an `application_users` membership, the IdP evaluates that environment's `enrollment_policy`: `consent_required` (default), `auto_join`, or `invite_only`.

## Considered Options

- **Always auto-provision silently**: Smooth for tightly coupled internal apps, but leaks profile data (`email`, `name`) to third-party applications without user awareness and breaks private/invite-only products.
- **Always require manual registration per app**: Defeats the seamless Ferrum-to-Elysiar SSO experience.
- **Per-environment `enrollment_policy` defaulting to `consent_required`**: Protects user privacy across distinct apps by default ("Continue to Elysiar as alice@example.com?"), allows first-party product suites to opt into zero-click `auto_join`, and supports closed/B2B apps with `invite_only`.

## Consequences

- Under `prompt=none`, if the user lacks an `application_users` row and the policy is `consent_required` or `invite_only`, `/oauth2/authorize` returns `error=consent_required` or `error=access_denied` instead of rendering UI.
- Creating the `application_users` row atomically assigns the environment's default role and emits `membership.created` to `outbox_events`.

