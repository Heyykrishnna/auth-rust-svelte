ALTER TABLE users ADD COLUMN IF NOT EXISTS primary_email_id UUID;
ALTER TABLE users ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

CREATE TABLE IF NOT EXISTS user_emails (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email       VARCHAR(320) NOT NULL,
    verified_at TIMESTAMPTZ,
    is_primary  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_emails_user_email_lower
    ON user_emails (user_id, LOWER(email));

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_emails_verified_email_lower
    ON user_emails (LOWER(email))
    WHERE verified_at IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_emails_primary_per_user
    ON user_emails (user_id)
    WHERE is_primary = TRUE;

CREATE INDEX IF NOT EXISTS idx_user_emails_user_id ON user_emails (user_id);
CREATE INDEX IF NOT EXISTS idx_user_emails_email_lower ON user_emails (LOWER(email));

CREATE TRIGGER user_emails_updated_at
    BEFORE UPDATE ON user_emails
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

INSERT INTO user_emails (user_id, email, verified_at, is_primary, created_at, updated_at)
SELECT
    id,
    LOWER(TRIM(email)),
    CASE WHEN email_verified THEN COALESCE(updated_at, created_at, NOW()) ELSE NULL END,
    TRUE,
    created_at,
    updated_at
FROM users
ON CONFLICT (user_id, LOWER(email)) DO NOTHING;

UPDATE users u
SET primary_email_id = ue.id
FROM user_emails ue
WHERE ue.user_id = u.id
  AND ue.is_primary = TRUE
  AND u.primary_email_id IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_users_primary_email'
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT fk_users_primary_email
            FOREIGN KEY (primary_email_id) REFERENCES user_emails(id) ON DELETE SET NULL;
    END IF;
END $$;

CREATE TRIGGER users_increment_version
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION increment_version();

CREATE TABLE IF NOT EXISTS oauth_accounts (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider                 VARCHAR(64) NOT NULL,
    provider_user_id         TEXT NOT NULL,
    email                    VARCHAR(320),
    email_verified           BOOLEAN NOT NULL DEFAULT FALSE,
    access_token_encrypted   BYTEA,
    refresh_token_encrypted  BYTEA,
    expires_at               TIMESTAMPTZ,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, provider_user_id),
    UNIQUE (user_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user_id ON oauth_accounts (user_id);
CREATE INDEX IF NOT EXISTS idx_oauth_accounts_provider_lookup ON oauth_accounts (provider, provider_user_id);

CREATE TRIGGER oauth_accounts_updated_at
    BEFORE UPDATE ON oauth_accounts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, email_verified, expires_at, created_at, updated_at)
SELECT
    user_id,
    provider::text,
    provider_user_id,
    email,
    FALSE,
    expires_at,
    created_at,
    updated_at
FROM oidc_accounts
ON CONFLICT (provider, provider_user_id) DO NOTHING;

CREATE TABLE IF NOT EXISTS sso_sessions (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token_hash  VARCHAR(128) NOT NULL UNIQUE,
    authenticated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    amr                 TEXT[] NOT NULL DEFAULT ARRAY['pwd']::TEXT[],
    acr                 VARCHAR(64) NOT NULL DEFAULT 'aal1',
    user_agent          TEXT,
    ip_address          INET,
    expires_at          TIMESTAMPTZ NOT NULL,
    last_seen_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_sso_sessions_expires CHECK (expires_at > created_at)
);

CREATE INDEX IF NOT EXISTS idx_sso_sessions_user_id ON sso_sessions (user_id);
CREATE INDEX IF NOT EXISTS idx_sso_sessions_active
    ON sso_sessions (user_id)
    WHERE revoked_at IS NULL;

CREATE TRIGGER sso_sessions_updated_at
    BEFORE UPDATE ON sso_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS application_users (
    id                UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id    UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status            VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'revoked')),
    external_id       VARCHAR(255),
    public_metadata   JSONB NOT NULL DEFAULT '{}'::jsonb,
    private_metadata  JSONB NOT NULL DEFAULT '{}'::jsonb,
    version           BIGINT NOT NULL DEFAULT 1,
    joined_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at    TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, user_id),
    UNIQUE (environment_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_application_users_env_external_id
    ON application_users (environment_id, external_id)
    WHERE external_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_application_users_user_id ON application_users (user_id);
CREATE INDEX IF NOT EXISTS idx_application_users_env_sync
    ON application_users (environment_id, updated_at, id);

CREATE TRIGGER application_users_updated_at
    BEFORE UPDATE ON application_users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER application_users_increment_version
    BEFORE UPDATE ON application_users
    FOR EACH ROW
    EXECUTE FUNCTION increment_version();

CREATE TABLE IF NOT EXISTS app_sessions (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id          UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    application_user_id     UUID NOT NULL,
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sso_session_id          UUID REFERENCES sso_sessions(id) ON DELETE SET NULL,
    session_hash            VARCHAR(128) NOT NULL,
    user_agent              TEXT,
    ip_address              INET,
    active_organization_id  UUID,
    expires_at              TIMESTAMPTZ NOT NULL,
    last_used_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at              TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, id),
    CONSTRAINT fk_app_sessions_application_user
        FOREIGN KEY (environment_id, application_user_id)
        REFERENCES application_users(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT chk_app_sessions_expires CHECK (expires_at > created_at)
);

CREATE INDEX IF NOT EXISTS idx_app_sessions_env_app_user
    ON app_sessions (environment_id, application_user_id);
CREATE INDEX IF NOT EXISTS idx_app_sessions_sso_session_id
    ON app_sessions (sso_session_id)
    WHERE sso_session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_app_sessions_active
    ON app_sessions (environment_id, application_user_id)
    WHERE revoked_at IS NULL;

CREATE TRIGGER app_sessions_updated_at
    BEFORE UPDATE ON app_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS environment_id UUID;
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS app_session_id UUID;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_refresh_tokens_app_session'
    ) THEN
        ALTER TABLE refresh_tokens
            ADD CONSTRAINT fk_refresh_tokens_app_session
            FOREIGN KEY (environment_id, app_session_id)
            REFERENCES app_sessions(environment_id, id)
            ON DELETE CASCADE;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_env_app_session
    ON refresh_tokens (environment_id, app_session_id)
    WHERE app_session_id IS NOT NULL;

