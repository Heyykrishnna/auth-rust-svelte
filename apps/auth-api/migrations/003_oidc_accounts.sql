-- Migration 003: OIDC accounts table
-- Links external provider identities to local users

CREATE TYPE oidc_provider AS ENUM ('google', 'github');

CREATE TABLE IF NOT EXISTS oidc_accounts (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider        oidc_provider NOT NULL,
    provider_user_id TEXT NOT NULL,     -- The user's ID on the provider (e.g., Google sub)
    email           VARCHAR(320),       -- Email from provider (may differ from user.email)
    access_token    TEXT,               -- Encrypted provider access token (optional)
    refresh_token   TEXT,               -- Encrypted provider refresh token (optional)
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One account per provider per user
    UNIQUE (user_id, provider),
    -- One provider account per provider user ID
    UNIQUE (provider, provider_user_id)
);

-- Index for provider lookups during OIDC callback
CREATE INDEX IF NOT EXISTS idx_oidc_accounts_provider ON oidc_accounts(provider, provider_user_id);

CREATE TRIGGER oidc_accounts_updated_at
    BEFORE UPDATE ON oidc_accounts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
