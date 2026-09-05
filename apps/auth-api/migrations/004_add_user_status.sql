-- Migration 004: Add user status for account status check
-- Ensures users have a status column ('active', 'suspended', 'deactivated')
-- for account status verification during authentication.

ALTER TABLE users ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'active';

CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
