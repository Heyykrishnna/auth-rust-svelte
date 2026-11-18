ALTER TABLE authentication_transactions
    ADD COLUMN IF NOT EXISTS issued_app_session_id UUID REFERENCES app_sessions(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS sso_session_id UUID REFERENCES sso_sessions(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS request_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_auth_tx_issued_app_session
    ON authentication_transactions (issued_app_session_id)
    WHERE issued_app_session_id IS NOT NULL;

