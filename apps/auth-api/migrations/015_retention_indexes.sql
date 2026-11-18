CREATE INDEX IF NOT EXISTS idx_audit_logs_retention ON audit_logs (created_at);
CREATE INDEX IF NOT EXISTS idx_outbox_events_retention ON outbox_events (updated_at) WHERE status IN ('published', 'dead_letter');
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_retention ON webhook_deliveries (updated_at) WHERE status IN ('succeeded', 'failed', 'dead_letter');
CREATE INDEX IF NOT EXISTS idx_authentication_transactions_expiry ON authentication_transactions (expires_at);
