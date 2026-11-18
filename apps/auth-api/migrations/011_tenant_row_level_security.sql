CREATE OR REPLACE FUNCTION current_environment_id()
RETURNS UUID AS $$
    SELECT NULLIF(current_setting('app.current_environment_id', true), '')::uuid;
$$ LANGUAGE sql STABLE;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'app_tenant_rls') THEN
        CREATE ROLE app_tenant_rls NOLOGIN NOBYPASSRLS;
    END IF;
END $$;

GRANT USAGE ON SCHEMA public TO app_tenant_rls;

GRANT SELECT ON
    developers,
    developer_workspaces,
    developer_workspace_members,
    applications,
    application_environments,
    users,
    user_emails,
    oauth_accounts,
    sso_sessions
TO app_tenant_rls;

GRANT SELECT, INSERT, UPDATE, DELETE ON
    application_domains,
    application_credentials,
    application_oauth_providers,
    application_users,
    app_sessions,
    refresh_tokens,
    organizations,
    organization_members,
    organization_invitations,
    environment_roles,
    environment_permissions,
    environment_role_permissions,
    application_user_roles,
    organization_member_roles,
    authentication_transactions,
    outbox_events,
    webhook_endpoints,
    webhook_deliveries,
    audit_logs
TO app_tenant_rls;

DO $$
DECLARE
    tbl TEXT;
    tenant_tables TEXT[] := ARRAY[
        'application_domains',
        'application_credentials',
        'application_oauth_providers',
        'application_users',
        'app_sessions',
        'organizations',
        'organization_members',
        'organization_invitations',
        'environment_roles',
        'environment_permissions',
        'environment_role_permissions',
        'application_user_roles',
        'organization_member_roles',
        'authentication_transactions',
        'outbox_events',
        'webhook_endpoints',
        'webhook_deliveries'
    ];
BEGIN
    FOREACH tbl IN ARRAY tenant_tables LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', tbl);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', tbl);
        EXECUTE format('DROP POLICY IF EXISTS tenant_isolation ON %I', tbl);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON %I USING (environment_id = current_environment_id()) WITH CHECK (environment_id = current_environment_id())',
            tbl
        );
    END LOOP;
END $$;

