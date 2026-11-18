CREATE TABLE IF NOT EXISTS signing_keys (
    id                     UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    kid                    VARCHAR(64) NOT NULL UNIQUE,
    algorithm              VARCHAR(16) NOT NULL CHECK (algorithm IN ('ES256', 'RS256')),
    public_jwk             JSONB NOT NULL,
    private_key_encrypted  BYTEA NOT NULL,
    kek_version            VARCHAR(32) NOT NULL,
    status                 VARCHAR(20) NOT NULL DEFAULT 'next' CHECK (status IN ('next', 'active', 'retiring', 'revoked')),
    activated_at           TIMESTAMPTZ,
    retires_at             TIMESTAMPTZ,
    revoked_at             TIMESTAMPTZ,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_signing_keys_status ON signing_keys (status);

CREATE TRIGGER signing_keys_updated_at
    BEFORE UPDATE ON signing_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS authentication_transactions (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id           UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    flow_type                VARCHAR(32) NOT NULL CHECK (flow_type IN ('oidc_authorize', 'social_oauth', 'email_otp', 'password_reset')),
    state_hash               VARCHAR(128),
    nonce                    VARCHAR(255),
    code_challenge           VARCHAR(255),
    code_challenge_method    VARCHAR(16),
    redirect_uri             TEXT,
    scope                    TEXT,
    authorization_code_hash  VARCHAR(128),
    user_id                  UUID REFERENCES users(id) ON DELETE CASCADE,
    status                   VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'authenticated', 'code_issued', 'consumed', 'expired', 'revoked')),
    expires_at               TIMESTAMPTZ NOT NULL,
    consumed_at              TIMESTAMPTZ,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_auth_tx_expires CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_tx_state_hash
    ON authentication_transactions (state_hash)
    WHERE state_hash IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_tx_code_hash
    ON authentication_transactions (authorization_code_hash)
    WHERE authorization_code_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_auth_tx_environment_id
    ON authentication_transactions (environment_id, expires_at);

CREATE TRIGGER authentication_transactions_updated_at
    BEFORE UPDATE ON authentication_transactions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS outbox_events (
    id                 UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id     UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    event_type         VARCHAR(100) NOT NULL,
    aggregate_type     VARCHAR(64) NOT NULL,
    aggregate_id       UUID NOT NULL,
    aggregate_version  BIGINT NOT NULL,
    payload            JSONB NOT NULL,
    status             VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'published', 'dead_letter')),
    available_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempts           INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, id)
);

CREATE INDEX IF NOT EXISTS idx_outbox_events_poll
    ON outbox_events (status, available_at)
    WHERE status IN ('pending', 'processing');

CREATE INDEX IF NOT EXISTS idx_outbox_events_env_created
    ON outbox_events (environment_id, created_at, id);

CREATE TRIGGER outbox_events_updated_at
    BEFORE UPDATE ON outbox_events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id                        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id            UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    url                       TEXT NOT NULL,
    description               TEXT NOT NULL DEFAULT '',
    subscribed_events         TEXT[] NOT NULL DEFAULT '{}',
    signing_secret_encrypted  BYTEA NOT NULL,
    kek_version               VARCHAR(32) NOT NULL,
    status                    VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'disabled')),
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, id)
);

CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_environment_id
    ON webhook_endpoints (environment_id);

CREATE TRIGGER webhook_endpoints_updated_at
    BEFORE UPDATE ON webhook_endpoints
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id                     UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id         UUID NOT NULL REFERENCES application_environments(id) ON DELETE CASCADE,
    webhook_endpoint_id    UUID NOT NULL,
    outbox_event_id        UUID NOT NULL,
    attempt_number         INTEGER NOT NULL DEFAULT 1 CHECK (attempt_number > 0),
    request_headers        JSONB NOT NULL DEFAULT '{}'::jsonb,
    response_status        INTEGER,
    response_body_excerpt  TEXT,
    latency_ms             INTEGER,
    status                 VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'succeeded', 'retrying', 'failed', 'dead_letter')),
    next_retry_at          TIMESTAMPTZ,
    delivered_at           TIMESTAMPTZ,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (webhook_endpoint_id, outbox_event_id, attempt_number),
    CONSTRAINT fk_webhook_deliveries_endpoint
        FOREIGN KEY (environment_id, webhook_endpoint_id)
        REFERENCES webhook_endpoints(environment_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_webhook_deliveries_outbox_event
        FOREIGN KEY (environment_id, outbox_event_id)
        REFERENCES outbox_events(environment_id, id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_retry
    ON webhook_deliveries (status, next_retry_at)
    WHERE status IN ('pending', 'retrying');

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_env_endpoint
    ON webhook_deliveries (environment_id, webhook_endpoint_id, created_at DESC);

CREATE TRIGGER webhook_deliveries_updated_at
    BEFORE UPDATE ON webhook_deliveries
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

ALTER TABLE audit_logs ALTER COLUMN event TYPE TEXT USING event::text;
DROP TYPE IF EXISTS auth_audit_event;

ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS environment_id UUID REFERENCES application_environments(id) ON DELETE SET NULL;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS actor_type VARCHAR(30) NOT NULL DEFAULT 'user';
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS actor_id UUID;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS resource_type VARCHAR(64);
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS resource_id UUID;

CREATE INDEX IF NOT EXISTS idx_audit_logs_env_created
    ON audit_logs (environment_id, created_at DESC)
    WHERE environment_id IS NOT NULL;

