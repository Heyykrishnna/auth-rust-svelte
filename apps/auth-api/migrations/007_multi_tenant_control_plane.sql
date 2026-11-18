CREATE OR REPLACE FUNCTION increment_version()
RETURNS TRIGGER AS $$
BEGIN
    NEW.version = OLD.version + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS developers (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email           VARCHAR(320) NOT NULL,
    password_hash   TEXT,
    display_name    VARCHAR(100) NOT NULL DEFAULT '',
    email_verified  BOOLEAN NOT NULL DEFAULT FALSE,
    status          VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_developers_email_lower ON developers (LOWER(email));
CREATE INDEX IF NOT EXISTS idx_developers_status ON developers (status);

CREATE TRIGGER developers_updated_at
    BEFORE UPDATE ON developers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS developer_workspaces (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug        VARCHAR(100) NOT NULL UNIQUE,
    name        VARCHAR(150) NOT NULL,
    status      VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER developer_workspaces_updated_at
    BEFORE UPDATE ON developer_workspaces
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS developer_workspace_members (
    workspace_id  UUID NOT NULL REFERENCES developer_workspaces(id) ON DELETE CASCADE,
    developer_id  UUID NOT NULL REFERENCES developers(id) ON DELETE CASCADE,
    role          VARCHAR(30) NOT NULL CHECK (role IN ('owner', 'admin', 'developer', 'viewer')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, developer_id)
);

CREATE INDEX IF NOT EXISTS idx_developer_workspace_members_developer_id
    ON developer_workspace_members (developer_id);

CREATE TRIGGER developer_workspace_members_updated_at
    BEFORE UPDATE ON developer_workspace_members
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS applications (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id  UUID NOT NULL REFERENCES developer_workspaces(id) ON DELETE CASCADE,
    slug          VARCHAR(100) NOT NULL UNIQUE,
    name          VARCHAR(150) NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    metadata      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_applications_workspace_id ON applications (workspace_id);
CREATE INDEX IF NOT EXISTS idx_applications_status ON applications (status);

CREATE TRIGGER applications_updated_at
    BEFORE UPDATE ON applications
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS application_environments (
    id                          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    application_id              UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    kind                        VARCHAR(20) NOT NULL CHECK (kind IN ('development', 'production')),
    slug                        VARCHAR(120) NOT NULL,
    enrollment_policy           VARCHAR(30) NOT NULL DEFAULT 'consent_required'
                                CHECK (enrollment_policy IN ('consent_required', 'auto_join', 'invite_only')),
    session_ttl_secs            INTEGER NOT NULL DEFAULT 604800 CHECK (session_ttl_secs > 0),
    access_token_ttl_secs       INTEGER NOT NULL DEFAULT 300 CHECK (access_token_ttl_secs BETWEEN 60 AND 86400),
    require_email_verification  BOOLEAN NOT NULL DEFAULT TRUE,
    require_mfa                 BOOLEAN NOT NULL DEFAULT FALSE,
    branding                    JSONB NOT NULL DEFAULT '{}'::jsonb,
    settings                    JSONB NOT NULL DEFAULT '{}'::jsonb,
    status                      VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (application_id, kind),
    UNIQUE (application_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_application_environments_application_id
    ON application_environments (application_id);

CREATE TRIGGER application_environments_updated_at
    BEFORE UPDATE ON application_environments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS application_domains (
    id                         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id             UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    domain                     VARCHAR(255) NOT NULL,
    is_verified                BOOLEAN NOT NULL DEFAULT FALSE,
    verification_token         TEXT,
    allowed_origins            TEXT[] NOT NULL DEFAULT '{}',
    redirect_uris              TEXT[] NOT NULL DEFAULT '{}',
    post_logout_redirect_uris  TEXT[] NOT NULL DEFAULT '{}',
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, domain)
);

CREATE INDEX IF NOT EXISTS idx_application_domains_environment_id
    ON application_domains (environment_id);

CREATE TRIGGER application_domains_updated_at
    BEFORE UPDATE ON application_domains
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS application_credentials (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id   UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    credential_type  VARCHAR(20) NOT NULL CHECK (credential_type IN ('publishable', 'secret')),
    key_prefix       VARCHAR(16) NOT NULL,
    public_key       VARCHAR(128),
    secret_hash      VARCHAR(128),
    name             VARCHAR(120) NOT NULL DEFAULT '',
    scopes           TEXT[] NOT NULL DEFAULT '{}',
    last_used_at     TIMESTAMPTZ,
    expires_at       TIMESTAMPTZ,
    revoked_at       TIMESTAMPTZ,
    created_by       UUID REFERENCES developers(id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_credential_key_shape CHECK (
        (credential_type = 'publishable' AND public_key IS NOT NULL AND secret_hash IS NULL)
        OR (credential_type = 'secret' AND secret_hash IS NOT NULL AND public_key IS NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_application_credentials_public_key
    ON application_credentials (public_key)
    WHERE public_key IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_application_credentials_secret_hash
    ON application_credentials (secret_hash)
    WHERE secret_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_application_credentials_environment_id
    ON application_credentials (environment_id);

CREATE TRIGGER application_credentials_updated_at
    BEFORE UPDATE ON application_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS application_oauth_providers (
    id                        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id            UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    provider                  VARCHAR(64) NOT NULL,
    enabled                   BOOLEAN NOT NULL DEFAULT FALSE,
    client_id                 TEXT,
    client_secret_encrypted   BYTEA,
    kek_version               VARCHAR(32),
    scopes                    TEXT[] NOT NULL DEFAULT '{}',
    use_platform_credentials  BOOLEAN NOT NULL DEFAULT TRUE,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_application_oauth_providers_environment_id
    ON application_oauth_providers (environment_id);

CREATE TRIGGER application_oauth_providers_updated_at
    BEFORE UPDATE ON application_oauth_providers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

