CREATE TABLE account_security_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    email_alerts BOOLEAN NOT NULL DEFAULT TRUE,
    totp_secret BYTEA,
    totp_key_version TEXT,
    totp_last_step BIGINT,
    CHECK ((totp_secret IS NULL) = (totp_key_version IS NULL)),
    CHECK (totp_secret IS NOT NULL OR totp_last_step IS NULL)
);

CREATE TABLE account_backup_codes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    PRIMARY KEY (user_id, code_hash)
);

CREATE TABLE account_passkeys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id TEXT NOT NULL UNIQUE,
    credential JSONB NOT NULL,
    sign_count BIGINT NOT NULL DEFAULT 0 CHECK (sign_count >= 0),
    label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_account_passkeys_user ON account_passkeys (user_id, created_at);

CREATE TABLE account_trusted_devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_account_trusted_devices_user ON account_trusted_devices (user_id, last_seen_at DESC);

CREATE TABLE account_ip_intelligence (
    network CIDR PRIMARY KEY,
    country TEXT CHECK (country ~ '^[A-Z]{2}$'),
    latitude DOUBLE PRECISION CHECK (latitude BETWEEN -90 AND 90),
    longitude DOUBLE PRECISION CHECK (longitude BETWEEN -180 AND 180),
    reputation TEXT CHECK (reputation IN ('elevated', 'high')),
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK ((latitude IS NULL) = (longitude IS NULL))
);

CREATE INDEX idx_account_ip_intelligence_network ON account_ip_intelligence USING gist (network inet_ops);

ALTER TABLE app_sessions
    ADD COLUMN auth_methods TEXT[] NOT NULL DEFAULT ARRAY['pwd']::TEXT[];
