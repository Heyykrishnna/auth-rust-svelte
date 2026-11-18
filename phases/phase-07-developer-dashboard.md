# Phase 07: Developer Control Plane Dashboard & Operational Management

- **Status:** Planned
- **Prerequisites:** Phase 02 (Dashboard Shell), Phase 03 (Auth/Sessions), Phase 04 (OIDC/SSO), Phase 05 (Webhooks) completed
- **Primary Targets:** `apps/web/src/routes/dashboard/*`, `apps/auth-api/src/routes/control*.rs`

---

## Objective

Expand the initial Phase 02 Control Plane shell into a complete, self-service SaaS Developer Dashboard where developers and teams can inspect users, manage sessions, configure authentication methods & OIDC/SSO policies, debug webhook deliveries/replays, inspect audit logs, and manage workspace teammates without touching internal infrastructure.

---

## 1. Dashboard Overview & Health

- Environment-scoped KPI cards (`Development` / `Production` clearly distinguished):
  - Total users in environment, active `app_sessions`, 24h/7d auth success & failure rates, failed login/lockout spikes, webhook delivery success rate, API key usage.
- Recent security & audit event feed (suspensions, refresh token reuse detections, key rotations, failed webhook deliveries).
- Quick integration panel showing environment `issuer_url`, `jwks_uri`, `publishable_key`, and copy-paste SvelteKit / Next.js / OIDC config snippets.

---

## 2. Authentication, OIDC & SSO Configuration UI

- **Authentication Methods:** Toggle email/password, email OTP verification requirement, password policy (min length, breach/strength rules), and future MFA enforcement toggles.
- **Social OAuth Providers:** Enable/disable Google & GitHub per environment; choose platform-managed credentials vs. custom encrypted client ID/secret with exact callback URL display.
- **OIDC & SSO Policy:** Configure `enrollment_policy` (`consent_required`, `auto_join`, `invite_only`), access token TTL, session TTL, allowed redirect URIs, and post-logout redirect URIs.
- **Hosted Portal Branding:** Configure app display name, logo URL, primary accent color, support email, and live preview of the Hosted Sign-In & Consent screen.

---

## 3. End-User & Session Management Directory

- **User Directory:** Paginated, searchable (`email`, `sub`, `app_user_id`, `external_id`), filterable (`status`, `email_verified`, `role`).
- **User Detail Drawer/Page:**
  - Global identity summary (verified emails, linked OAuth providers) + environment-specific membership details (`public_metadata` / `private_metadata` JSON editor, `external_id`, `version`, `joined_at`).
  - Actions: assign/revoke roles, suspend/restore user in this environment, trigger password reset email, force-revoke all `app_sessions` in this environment.
  - Active `app_sessions` table (device/user-agent, IP, created/last-used timestamps, one-click revoke).
  - User-scoped audit trail.

---

## 4. Webhooks & Sync Operations UI

- **Endpoint Management:** Add/edit/pause/delete webhook endpoints, select subscribed event types, copy/rotate signing secrets (`whsec_*`) with grace-window indicator.
- **Test & Debugging Tools:** One-click "Send Test Event" (`webhook.test`), filterable delivery log table (status code, latency, attempt count, `event_id`, `aggregate_version`), expandable request payload & response excerpt inspector.
- **Recovery Operations:** One-click replay of single failed/dead-letter deliveries and bulk time-window replay for recovering from consumer service outages.

---

## 5. Workspace, Team & Security Settings

- Workspace team member management: invite teammates, assign roles (`owner`, `admin`, `developer`, `viewer`), remove members, enforce role-based UI visibility and backend API enforcement.
- Environment-wide Audit Log Explorer: filter by actor, event type, resource, date range, and export CSV/JSON.
- Developer account security settings (password change, developer active sessions).

---

## Required Tests & Verification Gates (Executed in Phase 07)

- **Control Plane RBAC Tests:** Verify `viewer` can read dashboard pages/APIs but receives `403` on mutations (key rotation, user suspension, webhook edits, app deletion); `developer` cannot delete workspace or manage billing/owners.
- **Cross-Workspace & Cross-Environment Isolation Tests:** Verify Dashboard queries for User Directory, Sessions, Webhooks, and Audit Logs strictly enforce workspace ownership and active `environment_id`.
- **Frontend Component & Accessibility Tests:** Vitest + testing-library coverage for pagination, search/filter debouncing, error/empty/loading states, keyboard navigation, and accessible form labels/modals.
- **Responsive & SSR Hydration Checks:** Verify zero Svelte hydration warnings and usable layouts across desktop and mobile viewports.

---

## Deliverables

- Complete Developer Control Plane Backend APIs (`/v1/control/*`) for user directory, provider config, webhook inspection/replay, audit search, and team roles
- Complete Developer Control Plane Dashboard in `apps/web`
- Automated RBAC, isolation, and UI component test suites

---

## Acceptance Criteria

- [ ] Developers can fully configure authentication methods, social providers, OIDC/SSO policies, branding, API keys, domains, and webhooks purely through the Dashboard.
- [ ] Developers can search environment users, edit metadata/roles, inspect and revoke active sessions, inspect failed webhook deliveries, and replay them with one click.
- [ ] Workspace RBAC (`owner`, `admin`, `developer`, `viewer`) is enforced on every Control Plane backend endpoint and reflected accurately in the UI.
- [ ] All loading, empty, validation, and API error states are handled cleanly with accessible UI semantics.

---

## Out of Scope

- Organization management & custom role editor UI (Phase 08)
- Time-series analytics charts & Prometheus/Grafana integration (Phase 09)

