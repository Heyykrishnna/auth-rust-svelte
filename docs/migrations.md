# Database Migrations & Rollback Guide

`apps/auth-api` applies versioned SQL migrations sequentially at startup via `sqlx::migrate!("./migrations")`. Every migration file uses sequential numbering (`001`–`011`) and is designed to run cleanly both on a brand-new PostgreSQL 16 database and on an existing populated single-tenant installation.

---

## 1. Migration Inventory

| Version | File | Scope |
| --- | --- | --- |
| `001` | `001_initial_users.sql` | Initial `users` table and `update_updated_at()` trigger function |
| `002` | `002_sessions.sql` | Legacy single-tenant `sessions` table |
| `003` | `003_oidc_accounts.sql` | Legacy `oidc_accounts` table (`oidc_provider` enum) |
| `004` | `004_add_user_status.sql` | Account status column and index on `users` |
| `005` | `005_token_families_and_audit_logs.sql` | `refresh_tokens` families and `audit_logs` |
| `006` | `006_roles_and_permissions.sql` | Legacy single-tenant `roles` and `permissions` |
| `007` | `007_multi_tenant_control_plane.sql` | `increment_version()`, `developers`, `developer_workspaces`, `developer_workspace_members`, `applications`, `application_environments`, `application_domains`, `application_credentials`, `application_oauth_providers` |
| `008` | `008_global_identity_and_memberships.sql` | `users.primary_email_id` + `version` trigger, `user_emails` (+ backfill), `oauth_accounts` (+ backfill), `sso_sessions`, `application_users`, `app_sessions` (composite FK), `refresh_tokens` multi-tenant columns |
| `009` | `009_organizations_and_tenant_rbac.sql` | `organizations`, `organization_members`, `organization_invitations`, `environment_roles`, `environment_permissions`, `environment_role_permissions`, `application_user_roles`, `organization_member_roles` (all composite-FK isolated) |
| `010` | `010_platform_infrastructure_and_audit.sql` | `signing_keys`, `authentication_transactions`, `outbox_events`, `webhook_endpoints`, `webhook_deliveries`, converts `audit_logs.event` from ENUM to `TEXT` and adds `environment_id` / actor / resource columns |
| `011` | `011_tenant_row_level_security.sql` | `current_environment_id()`, `app_tenant_rls` (`NOLOGIN NOBYPASSRLS`) role, grants, and `FORCE ROW LEVEL SECURITY` (`tenant_isolation` policy) on all 17 tenant-scoped tables |

---

## 2. Backward Compatibility & Single-Tenant Backfill

- Migration `008` automatically backfills every existing `users` row into `user_emails` (preserving verified status and linking `users.primary_email_id`) and copies any existing `oidc_accounts` into `oauth_accounts` (`provider` converted to `TEXT`).
- Legacy `sessions`, `roles`, `permissions`, `user_roles`, and `user_permissions` remain intact during Phases 01–02 so existing single-tenant routes continue working until cutover in Phase 03.
- `users_increment_version` is attached **after** the `primary_email_id` backfill so pre-existing users start cleanly at `version = 1`.

---

## 3. Running & Validating Migrations Locally

```bash
# Run migrations against PostgreSQL 16
export TEST_DATABASE_URL="postgres://authuser:authpassword@127.0.0.1:5432/auth_test"

# Execute live migration, composite-FK, monotonic-version, and concurrent RLS tests
cargo test --manifest-path apps/auth-api/Cargo.toml \
  --test tenant_isolation_tests -- --ignored --test-threads=1
```

---

## 4. Ordered Rollback Procedure (`011` -> `007`)

Always take a verified `pg_dump` snapshot before rolling back a production database. Execute rollbacks strictly in reverse order (`011` -> `010` -> `009` -> `008` -> `007`) and remove matching rows from `_sqlx_migrations` only after each step succeeds:

### Rollback `011_tenant_row_level_security.sql`
```sql
DO $$
DECLARE
    tbl TEXT;
    tenant_tables TEXT[] := ARRAY[
        'application_domains', 'application_credentials', 'application_oauth_providers',
        'application_users', 'app_sessions', 'organizations', 'organization_members',
        'organization_invitations', 'environment_roles', 'environment_permissions',
        'environment_role_permissions', 'application_user_roles', 'organization_member_roles',
        'authentication_transactions', 'outbox_events', 'webhook_endpoints', 'webhook_deliveries'
    ];
BEGIN
    FOREACH tbl IN ARRAY tenant_tables LOOP
        EXECUTE format('DROP POLICY IF EXISTS tenant_isolation ON %I', tbl);
        EXECUTE format('ALTER TABLE %I NO FORCE ROW LEVEL SECURITY', tbl);
        EXECUTE format('ALTER TABLE %I DISABLE ROW LEVEL SECURITY', tbl);
    END LOOP;
END $$;

DROP FUNCTION IF EXISTS current_environment_id();
DELETE FROM _sqlx_migrations WHERE version = 11;
```

### Rollback `010_platform_infrastructure_and_audit.sql`
```sql
DROP TABLE IF EXISTS webhook_deliveries;
DROP TABLE IF EXISTS webhook_endpoints;
DROP TABLE IF EXISTS outbox_events;
DROP TABLE IF EXISTS authentication_transactions;
DROP TABLE IF EXISTS signing_keys;

DROP INDEX IF EXISTS idx_audit_logs_env_created;
ALTER TABLE audit_logs
    DROP COLUMN IF EXISTS resource_id,
    DROP COLUMN IF EXISTS resource_type,
    DROP COLUMN IF EXISTS actor_id,
    DROP COLUMN IF EXISTS actor_type,
    DROP COLUMN IF EXISTS environment_id;

DELETE FROM _sqlx_migrations WHERE version = 10;
```

### Rollback `009_organizations_and_tenant_rbac.sql`
```sql
ALTER TABLE app_sessions DROP CONSTRAINT IF EXISTS fk_app_sessions_active_org;
DROP TABLE IF EXISTS organization_invitations;
DROP TABLE IF EXISTS organization_member_roles;
DROP TABLE IF EXISTS application_user_roles;
DROP TABLE IF EXISTS environment_role_permissions;
DROP TABLE IF EXISTS environment_permissions;
DROP TABLE IF EXISTS environment_roles;
DROP TABLE IF EXISTS organization_members;
DROP TABLE IF EXISTS organizations;
DELETE FROM _sqlx_migrations WHERE version = 9;
```

### Rollback `008_global_identity_and_memberships.sql`
```sql
ALTER TABLE refresh_tokens DROP CONSTRAINT IF EXISTS fk_refresh_tokens_app_session;
ALTER TABLE refresh_tokens
    DROP COLUMN IF EXISTS app_session_id,
    DROP COLUMN IF EXISTS environment_id;

DROP TABLE IF EXISTS app_sessions;
DROP TABLE IF EXISTS application_users;
DROP TABLE IF EXISTS sso_sessions;
DROP TABLE IF EXISTS oauth_accounts;

DROP TRIGGER IF EXISTS users_increment_version ON users;
ALTER TABLE users DROP CONSTRAINT IF EXISTS fk_users_primary_email;
ALTER TABLE users
    DROP COLUMN IF EXISTS primary_email_id,
    DROP COLUMN IF EXISTS version;
DROP TABLE IF EXISTS user_emails;

DELETE FROM _sqlx_migrations WHERE version = 8;
```

### Rollback `007_multi_tenant_control_plane.sql`
```sql
DROP TABLE IF EXISTS application_oauth_providers;
DROP TABLE IF EXISTS application_credentials;
DROP TABLE IF EXISTS application_domains;
DROP TABLE IF EXISTS application_environments;
DROP TABLE IF EXISTS applications;
DROP TABLE IF EXISTS developer_workspace_members;
DROP TABLE IF EXISTS developer_workspaces;
DROP TABLE IF EXISTS developers;
DROP FUNCTION IF EXISTS increment_version();
DELETE FROM _sqlx_migrations WHERE version = 7;
```

