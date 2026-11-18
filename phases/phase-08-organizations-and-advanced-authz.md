# Phase 08: Organizations, Invitations, Custom RBAC & Advanced Identity

- **Status:** Planned
- **Prerequisites:** Phase 05 (Outbox/Webhooks), Phase 06 (SDKs), Phase 07 (Developer Dashboard) completed
- **Primary Targets:** `apps/auth-api/src/services/organizations.rs`, `apps/auth-api/src/services/authorization.rs`, `apps/auth-api/src/routes/`, `packages/*`, `apps/web/`

---

## Objective

Extend the platform to support B2B / multi-team SaaS applications—giving consuming apps (like Ferrum or Elysiar) turnkey **Organizations**, secure **Member Invitations**, **Custom Roles & Permissions**, **Active-Organization Token Context**, **Multi-Email / Multi-OAuth Identity Management**, and full **Organization Webhook & Pull Synchronization**.

---

## 1. Organization Lifecycle & Membership Services

- **Organization CRUD (Environment-Scoped):**
  - Create, update profile (`name`, `slug`, `logo_url`, `metadata`), suspend, and delete organizations within an `environment_id`.
  - Unique `(environment_id, slug)` enforcement; atomic creation assigns creator as initial organization `owner` (must be an active `application_user` in the same environment).
- **Invitation Workflow (Secure & Replay-Proof):**
  - Create invitation (`email`, `role_id`, `expires_at` default 7 days), store hashed single-use `token_hash` in `organization_invitations`.
  - Deliver branded invitation email pointing to consuming app or Hosted Portal acceptance flow.
  - Acceptance validates: token exists, not expired, not revoked, not already accepted, and the accepting user's **verified email** matches the invited email (or explicit admin override policy).
  - Atomically mark invitation `accepted`, ensure `application_users` membership exists, insert `organization_members` + role, and emit `organization.member_added` outbox event.
- **Membership & Ownership Management:**
  - List members, update member role, remove member, and **safe ownership transfer** (prevent removing/demoting the last organization `owner`).
- **Active Organization Switching on `app_sessions`:**
  - `POST /v1/client/sessions/active-organization` verifies membership in the target org, updates `app_sessions.active_organization_id`, and mints a refreshed `access_token` carrying `org_id`, `org_slug`, `org_role`, and effective `org_permissions` claims.

---

## 2. Custom Roles & Advanced Authorization Engine

- **Custom Roles & Permissions per Environment:**
  - Developers define custom permissions (`billing.manage`, `projects.write`, `reports.export`) and custom roles (`org:owner`, `org:admin`, `org:billing`, `app:moderator`) scoped to `environment_id`.
  - Support both **application-level permissions** (from `application_user_roles`) and **organization-level permissions** (from `organization_member_roles` scoped to `active_organization_id`).
- **Permission Resolution & Evaluation:**
  - Deterministic permission union with strict scope boundaries (organization permissions apply **only** when `active_organization_id` matches the target resource's organization).
  - Role assignment audit history recorded in `audit_logs`.
  - Refresh tokens and active-org switches re-resolve permissions dynamically so revoked permissions take effect immediately at the next short-lived token refresh.

---

## 3. Advanced Global Identity Management

- **Multiple Verified Emails (`user_emails`):** Add email -> verify via OTP -> promote to primary email -> remove non-primary email (prevent removing the last verified email).
- **Multiple Linked OAuth Accounts (`oauth_accounts`):** Link additional providers (e.g., add GitHub to an existing Google/password account) and unlink providers (prevent unlinking the user's sole authentication method if no password is set).
- **Account Recovery, Export & Deletion:**
  - Self-service identity data export (GDPR-friendly JSON bundle of global profile, emails, linked OAuth metadata, memberships, and recent audit events).
  - Account deletion workflow with grace period / immediate revocation of `sso_sessions` and all `app_sessions`, emitting `user.deleted` to every joined environment so consuming services can clean up or anonymize local records.

---

## 4. Organization Sync Events, Backend API & SDK/Dashboard Updates

- **Webhook Events (via Phase 05 Outbox):**
  - `organization.created`, `organization.updated`, `organization.deleted`
  - `organization.member_added`, `organization.member_updated`, `organization.member_removed`
  - `role.assigned`, `role.revoked`
- **Backend Reconciliation Endpoints (`/v1/backend/organizations`):**
  - Paginated `GET /v1/backend/organizations?updated_since=...`, `GET /v1/backend/organizations/:org_id/members`, plus server-side org/member creation & role updates.
- **SDK Extensions (`@secure-auth/core`, `svelte`, `react`, `next`):**
  - `useOrganization()`, `useOrganizationList()`, `<OrganizationSwitcher />`, `<OrganizationProfile />`, typed org webhook handlers, and server helpers checking `has({ permission, role, orgId })`.
- **Developer Dashboard Extensions (`apps/web`):**
  - Organizations directory, org settings & member list, invitation manager, Role & Permission Editor UI, and org-scoped audit log view.

---

## Required Tests & Verification Gates (Executed in Phase 08)

- **Organization Isolation Tests:** Member of Org A in Environment 1 cannot read, invite, mutate, or switch into Org B (same environment) or any Org in Environment 2.
- **Invitation Security Tests:** Expired invitation, revoked invitation, replayed accepted invitation, and mismatched unverified email acceptance all fail cleanly.
- **Lockout & Last-Owner Protection Tests:** Attempting to remove/demote the final org owner, remove the final verified email, or unlink the final login method (without a password) is rejected.
- **Org Sync & Claim Tests:** Switching active organization updates `org_id` and `org_permissions` in the newly minted JWT; org mutations emit monotonic-versioned outbox events and appear in `/v1/backend/organizations?updated_since=...`.

---

## Deliverables

- Organization, invitation, and custom RBAC services and APIs (`/v1/client/organizations/*`, `/v1/backend/organizations/*`, `/v1/control/.../roles`)
- Multi-email, multi-OAuth linking/unlinking, and account export/deletion flows
- Organization webhook events & pull reconciliation endpoints
- SDK organization hooks/components and Dashboard Org + Role Editor UI

---

## Acceptance Criteria

- [ ] Consuming B2B apps can create organizations, invite members with single-use expiring tokens, switch active organization context in JWTs, and evaluate custom org/app permissions.
- [ ] Organization and membership changes reliably synchronize to consuming services via both HMAC-signed webhooks and `/v1/backend/organizations` reconciliation endpoints.
- [ ] Users can safely manage multiple verified emails, link/unlink multiple OAuth providers, export their data, and delete their account with proper `user.deleted` fan-out.
- [ ] All Phase 08 isolation, invitation replay, last-owner, and sync tests pass.

---

## Out of Scope

- Enterprise SAML 2.0 / SCIM directory sync (post-v1 roadmap extension)
- Usage metering and billing enforcement (Phase 12)

