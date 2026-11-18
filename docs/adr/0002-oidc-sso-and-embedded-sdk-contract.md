---
status: accepted
---

# 0002: Hosted OIDC Redirect for Cross-App SSO vs. Embedded SDK Components

We standardize on **Hosted OIDC Authorization Code Flow with PKCE (`S256`)** on the Central Identity Provider origin as the canonical cross-application SSO mechanism, while supporting embedded SDK UI components (`<SignIn />`, `<SignUp />`) strictly as **app-local authentication** (unless configured in redirect mode).

## Considered Options

- **Third-party cookies from embedded `<SignIn />` forms**: Fails reliably across Safari ITP, Firefox ETP, and modern Chrome third-party cookie restrictions when Ferrum (`ferrum.com`) calls `auth.platform.com` via `fetch()`.
- **Per-app CNAME Frontend API domains only**: Works well for single-app first-party cookies, but isolates cookies per root domain and prevents seamless SSO between unrelated applications (`Ferrum` -> `Elysiar`).
- **Hosted OIDC Redirect (primary SSO) + Embedded SDK (app-local option)**: Top-level redirects keep the IdP `__Host-idp_sso_session` cookie strictly first-party (`HttpOnly`, `Secure`, `SameSite=Lax`), enabling reliable cross-domain SSO while still letting single-app developers embed custom forms when SSO is not needed.

## Consequences

- A dedicated **Hosted Auth Portal** surface is required on the IdP origin for sign-in, OTP verification, consent, and global logout, protected with `frame-ancestors 'none'`.
- SDK `<SignIn />` components default to `mode="redirect"` for SSO-enabled apps and explicitly document that `mode="embedded"` creates an `app_session` without establishing a cross-domain `sso_session`.

