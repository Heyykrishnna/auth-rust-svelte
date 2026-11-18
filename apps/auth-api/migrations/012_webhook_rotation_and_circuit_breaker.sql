ALTER TABLE webhook_endpoints
    ADD COLUMN IF NOT EXISTS previous_signing_secret_encrypted BYTEA,
    ADD COLUMN IF NOT EXISTS previous_secret_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK (consecutive_failures >= 0),
    ADD COLUMN IF NOT EXISTS paused_reason TEXT,
    ADD COLUMN IF NOT EXISTS last_failure_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_success_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_due
    ON webhook_deliveries (status, next_retry_at, created_at)
    WHERE status IN ('pending', 'retrying');
