---
status: proposed
---

# 0003: Dedicated Developer Control Plane Identity with Workspaces

We store Control Plane operators in a dedicated `developers` table paired with `developer_workspaces` and `developer_workspace_members` (`owner`, `admin`, `developer`, `viewer`), keeping Developer Control Plane authentication and sessions isolated from end-user `users` and `sso_sessions`.

## Considered Options

- **Dogfood global `users` with a reserved System Control Plane application**: Eliminates a second credential table and lets developers reuse their end-user login, but creates a bootstrap chicken-and-egg problem during migrations and couples Control Plane blast radius to end-user auth flows.
- **Dedicated `developers` + `developer_workspaces` tables**: Matches the reference ER separation (`DEVELOPERS -> APPLICATIONS`), isolates blast radius so an end-user vulnerability cannot escalate into Control Plane privileges, and supports multi-developer teams cleanly from Phase 01.

## Consequences

- Control Plane sessions use separate tokens/cookies and signing audience (`control-plane`) from end-user OIDC tokens.
- Both `developers` and `users` share the same hardened Argon2id, rate-limiting, and audit primitives internally without sharing database tables or session stores.
- Marked `proposed` pending final confirmation before Phase 01 SQL migrations are written.

