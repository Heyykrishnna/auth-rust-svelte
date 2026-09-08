-- Migration 005: Token families and audit logs
-- Implements refresh token rotation with family tracking, session revoking,
-- and append-only audit logging for security events.

-- 1. Extend sessions table with session_hash and revoked_at
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS session_hash VARCHAR(64);
UPDATE sessions SET session_hash = refresh_token_hash WHERE session_hash IS NULL;
ALTER TABLE sessions ALTER COLUMN session_hash SET DEFAULT '';
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_sessions_session_hash ON sessions(session_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(user_id) WHERE revoked_at IS NULL;

-- 2. Create refresh_tokens table for RFC 6819 token rotation with family_id
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id  UUID REFERENCES sessions(id) ON DELETE SET NULL,
    token_hash  VARCHAR(64) NOT NULL UNIQUE,
    family_id   UUID NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_refresh_tokens_expires CHECK (expires_at > created_at),
    CONSTRAINT chk_refresh_tokens_revoked CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family_id ON refresh_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active_family ON refresh_tokens(family_id) WHERE revoked_at IS NULL;

-- 3. Create auth_audit_event enum and audit_logs table
DO $$ BEGIN
    CREATE TYPE auth_audit_event AS ENUM (
        'LOGIN_SUCCESS',
        'LOGIN_FAILED',
        'LOGOUT',
        'PASSWORD_CHANGED',
        'EMAIL_VERIFIED',
        'SESSION_REVOKED',
        'MFA_ENABLED',
        'MFA_FAILED',
        'ACCOUNT_LOCKED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS audit_logs (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    event       auth_audit_event NOT NULL,
    ip_address  VARCHAR(45),
    user_agent  TEXT,
    metadata    JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id_created ON audit_logs(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_created ON audit_logs(event, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at DESC);
