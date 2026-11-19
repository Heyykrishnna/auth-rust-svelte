CREATE TABLE IF NOT EXISTS user_security_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    two_factor_method VARCHAR(32) NOT NULL DEFAULT 'totp',
    recovery_email VARCHAR(320),
    recovery_phone VARCHAR(50),
    security_notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    notify_new_login BOOLEAN NOT NULL DEFAULT TRUE,
    notify_suspicious_activity BOOLEAN NOT NULL DEFAULT TRUE,
    notify_security_changes BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER user_security_settings_updated_at
    BEFORE UPDATE ON user_security_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS user_totp_credentials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_base32 TEXT NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

CREATE TRIGGER user_totp_credentials_updated_at
    BEFORE UPDATE ON user_totp_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS user_passkeys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id TEXT NOT NULL UNIQUE,
    public_key_spki TEXT NOT NULL,
    name VARCHAR(100) NOT NULL,
    aaguid VARCHAR(64),
    sign_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_user_passkeys_user_id ON user_passkeys(user_id);

CREATE TABLE IF NOT EXISTS user_backup_codes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash VARCHAR(64) NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_backup_codes_user_id ON user_backup_codes(user_id);

CREATE TABLE IF NOT EXISTS user_devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    fingerprint VARCHAR(64) NOT NULL,
    device_name VARCHAR(120) NOT NULL,
    device_type VARCHAR(30) NOT NULL DEFAULT 'browser',
    trusted BOOLEAN NOT NULL DEFAULT FALSE,
    trusted_until TIMESTAMPTZ,
    last_ip VARCHAR(45),
    last_location VARCHAR(120),
    last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, fingerprint)
);

CREATE INDEX IF NOT EXISTS idx_user_devices_user_id ON user_devices(user_id);

CREATE TABLE IF NOT EXISTS user_security_alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    alert_type VARCHAR(64) NOT NULL,
    severity VARCHAR(20) NOT NULL DEFAULT 'medium',
    title VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    ip_address VARCHAR(45),
    location VARCHAR(120),
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_security_alerts_user ON user_security_alerts(user_id, resolved, created_at DESC);

