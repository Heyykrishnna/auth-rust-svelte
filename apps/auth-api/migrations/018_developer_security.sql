CREATE TABLE developer_sessions (
    id UUID PRIMARY KEY,
    developer_id UUID NOT NULL REFERENCES developers(id) ON DELETE CASCADE,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_developer_sessions_owner_created
    ON developer_sessions (developer_id, created_at DESC) WHERE revoked_at IS NULL;
CREATE INDEX idx_developer_sessions_expiry ON developer_sessions (expires_at);
CREATE INDEX idx_audit_logs_developer_security
    ON audit_logs (actor_id, created_at DESC)
    WHERE actor_type = 'developer' AND resource_type = 'developer';
