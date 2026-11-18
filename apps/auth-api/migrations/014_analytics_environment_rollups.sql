ALTER TABLE outbox_events
    ADD COLUMN IF NOT EXISTS traceparent TEXT;

CREATE TABLE IF NOT EXISTS analytics_environment_rollups (
    environment_id UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    bucket_granularity TEXT NOT NULL CHECK (bucket_granularity IN ('hour', 'day')),
    bucket_start TIMESTAMPTZ NOT NULL,
    registrations_count BIGINT NOT NULL DEFAULT 0,
    active_users_count BIGINT NOT NULL DEFAULT 0,
    sign_ins_success BIGINT NOT NULL DEFAULT 0,
    sign_ins_failure BIGINT NOT NULL DEFAULT 0,
    sign_ins_password BIGINT NOT NULL DEFAULT 0,
    sign_ins_google BIGINT NOT NULL DEFAULT 0,
    sign_ins_github BIGINT NOT NULL DEFAULT 0,
    sign_ins_oidc_sso BIGINT NOT NULL DEFAULT 0,
    lockouts_count BIGINT NOT NULL DEFAULT 0,
    sessions_created BIGINT NOT NULL DEFAULT 0,
    sessions_revoked BIGINT NOT NULL DEFAULT 0,
    refresh_reuse_detected BIGINT NOT NULL DEFAULT 0,
    client_api_requests BIGINT NOT NULL DEFAULT 0,
    backend_api_requests BIGINT NOT NULL DEFAULT 0,
    webhook_deliveries_succeeded BIGINT NOT NULL DEFAULT 0,
    webhook_deliveries_failed BIGINT NOT NULL DEFAULT 0,
    webhook_deliveries_dead_letter BIGINT NOT NULL DEFAULT 0,
    webhook_latency_sum_ms BIGINT NOT NULL DEFAULT 0,
    webhook_latency_count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (environment_id, bucket_granularity, bucket_start)
);

CREATE INDEX IF NOT EXISTS idx_analytics_env_rollups_lookup
    ON analytics_environment_rollups (environment_id, bucket_granularity, bucket_start DESC);

GRANT SELECT, INSERT, UPDATE, DELETE ON analytics_environment_rollups TO app_tenant_rls;

ALTER TABLE analytics_environment_rollups ENABLE ROW LEVEL SECURITY;
ALTER TABLE analytics_environment_rollups FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_isolation ON analytics_environment_rollups;
CREATE POLICY tenant_isolation ON analytics_environment_rollups
    USING (environment_id = current_environment_id())
    WITH CHECK (environment_id = current_environment_id());

