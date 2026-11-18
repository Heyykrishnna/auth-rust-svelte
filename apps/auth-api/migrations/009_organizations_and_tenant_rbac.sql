CREATE TABLE IF NOT EXISTS organizations (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id  UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    slug            VARCHAR(120) NOT NULL,
    name            VARCHAR(150) NOT NULL,
    logo_url        TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    version         BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, slug),
    UNIQUE (environment_id, id)
);

CREATE INDEX IF NOT EXISTS idx_organizations_env_sync
    ON organizations (environment_id, updated_at, id);

CREATE TRIGGER organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER organizations_increment_version
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION increment_version();

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_app_sessions_active_org'
    ) THEN
        ALTER TABLE app_sessions
            ADD CONSTRAINT fk_app_sessions_active_org
            FOREIGN KEY (environment_id, active_organization_id)
            REFERENCES organizations(environment_id, id)
            ON DELETE SET NULL (active_organization_id);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS organization_members (
    id                   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id       UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    organization_id      UUID NOT NULL,
    application_user_id  UUID NOT NULL,
    status               VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'removed')),
    version              BIGINT NOT NULL DEFAULT 1,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, organization_id, application_user_id),
    UNIQUE (environment_id, id),
    CONSTRAINT fk_organization_members_org
        FOREIGN KEY (environment_id, organization_id)
        REFERENCES organizations(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_organization_members_app_user
        FOREIGN KEY (environment_id, application_user_id)
        REFERENCES application_users(environment_id, id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_organization_members_env_org
    ON organization_members (environment_id, organization_id);
CREATE INDEX IF NOT EXISTS idx_organization_members_env_app_user
    ON organization_members (environment_id, application_user_id);

CREATE TRIGGER organization_members_updated_at
    BEFORE UPDATE ON organization_members
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER organization_members_increment_version
    BEFORE UPDATE ON organization_members
    FOR EACH ROW
    EXECUTE FUNCTION increment_version();

CREATE TABLE IF NOT EXISTS environment_roles (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id  UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    slug            VARCHAR(80) NOT NULL,
    name            VARCHAR(120) NOT NULL,
    scope           VARCHAR(20) NOT NULL DEFAULT 'application' CHECK (scope IN ('application', 'organization')),
    description     TEXT NOT NULL DEFAULT '',
    is_system       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, slug),
    UNIQUE (environment_id, id)
);

CREATE INDEX IF NOT EXISTS idx_environment_roles_environment_id
    ON environment_roles (environment_id);

CREATE TRIGGER environment_roles_updated_at
    BEFORE UPDATE ON environment_roles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS environment_permissions (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id  UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    slug            VARCHAR(120) NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, slug),
    UNIQUE (environment_id, id)
);

CREATE INDEX IF NOT EXISTS idx_environment_permissions_environment_id
    ON environment_permissions (environment_id);

CREATE TABLE IF NOT EXISTS environment_role_permissions (
    environment_id  UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    role_id         UUID NOT NULL,
    permission_id   UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (environment_id, role_id, permission_id),
    CONSTRAINT fk_env_role_permissions_role
        FOREIGN KEY (environment_id, role_id)
        REFERENCES environment_roles(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_env_role_permissions_permission
        FOREIGN KEY (environment_id, permission_id)
        REFERENCES environment_permissions(environment_id, id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS application_user_roles (
    environment_id       UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    application_user_id  UUID NOT NULL,
    role_id              UUID NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (environment_id, application_user_id, role_id),
    CONSTRAINT fk_app_user_roles_app_user
        FOREIGN KEY (environment_id, application_user_id)
        REFERENCES application_users(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_app_user_roles_role
        FOREIGN KEY (environment_id, role_id)
        REFERENCES environment_roles(environment_id, id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_application_user_roles_env_app_user
    ON application_user_roles (environment_id, application_user_id);

CREATE TABLE IF NOT EXISTS organization_member_roles (
    environment_id          UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    organization_member_id  UUID NOT NULL,
    role_id                 UUID NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (environment_id, organization_member_id, role_id),
    CONSTRAINT fk_org_member_roles_member
        FOREIGN KEY (environment_id, organization_member_id)
        REFERENCES organization_members(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_org_member_roles_role
        FOREIGN KEY (environment_id, role_id)
        REFERENCES environment_roles(environment_id, id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_organization_member_roles_env_member
    ON organization_member_roles (environment_id, organization_member_id);

CREATE TABLE IF NOT EXISTS organization_invitations (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id          UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    organization_id         UUID NOT NULL,
    email                   VARCHAR(320) NOT NULL,
    role_id                 UUID,
    token_hash              VARCHAR(128) NOT NULL UNIQUE,
    invited_by_app_user_id  UUID,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'revoked', 'expired')),
    expires_at              TIMESTAMPTZ NOT NULL,
    accepted_at             TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_org_invitations_org
        FOREIGN KEY (environment_id, organization_id)
        REFERENCES organizations(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_org_invitations_role
        FOREIGN KEY (environment_id, role_id)
        REFERENCES environment_roles(environment_id, id)
        ON DELETE SET NULL (role_id),
    CONSTRAINT fk_org_invitations_inviter
        FOREIGN KEY (environment_id, invited_by_app_user_id)
        REFERENCES application_users(environment_id, id)
        ON DELETE SET NULL (invited_by_app_user_id),
    CONSTRAINT chk_org_invitations_expires CHECK (expires_at > created_at)
);

CREATE INDEX IF NOT EXISTS idx_organization_invitations_env_org
    ON organization_invitations (environment_id, organization_id);
CREATE INDEX IF NOT EXISTS idx_organization_invitations_email_lower
    ON organization_invitations (environment_id, LOWER(email));

CREATE TRIGGER organization_invitations_updated_at
    BEFORE UPDATE ON organization_invitations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

