CREATE TABLE developer_security_settings (
    developer_id UUID PRIMARY KEY REFERENCES developers(id) ON DELETE CASCADE,
    totp_secret BYTEA,
    totp_key_version TEXT,
    totp_last_step BIGINT,
    notify_new_sign_in BOOLEAN NOT NULL DEFAULT FALSE,
    notify_suspicious_sign_in BOOLEAN NOT NULL DEFAULT TRUE,
    CHECK (
        (totp_secret IS NULL AND totp_key_version IS NULL AND totp_last_step IS NULL)
        OR (totp_secret IS NOT NULL AND totp_key_version IS NOT NULL AND totp_last_step IS NOT NULL)
    )
);

CREATE TABLE developer_backup_codes (
    developer_id UUID NOT NULL REFERENCES developers(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    PRIMARY KEY (developer_id, code_hash)
);

CREATE TABLE developer_passkeys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    developer_id UUID NOT NULL REFERENCES developers(id) ON DELETE CASCADE,
    credential_id TEXT NOT NULL UNIQUE,
    credential JSONB NOT NULL,
    sign_count BIGINT NOT NULL DEFAULT 0 CHECK (sign_count >= 0),
    label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX idx_developer_passkeys_owner ON developer_passkeys (developer_id, created_at);

CREATE TABLE developer_trusted_devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    developer_id UUID NOT NULL REFERENCES developers(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_developer_trusted_devices_owner ON developer_trusted_devices (developer_id, last_seen_at DESC);
CREATE INDEX idx_developer_trusted_devices_expiry ON developer_trusted_devices (expires_at);

ALTER TABLE developer_sessions
    ADD COLUMN verified_until TIMESTAMPTZ,
    ADD COLUMN trusted_device_id UUID REFERENCES developer_trusted_devices(id) ON DELETE SET NULL;
CREATE INDEX idx_developer_sessions_trusted_device ON developer_sessions (trusted_device_id)
    WHERE trusted_device_id IS NOT NULL;

CREATE TABLE developer_password_resets (
    developer_id UUID PRIMARY KEY REFERENCES developers(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_developer_password_resets_expiry ON developer_password_resets (expires_at);
