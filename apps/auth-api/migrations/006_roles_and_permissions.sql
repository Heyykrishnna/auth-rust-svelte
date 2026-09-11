CREATE TABLE IF NOT EXISTS roles (
    id          VARCHAR(50) PRIMARY KEY,
    name        VARCHAR(50) NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS permissions (
    id          VARCHAR(100) PRIMARY KEY,
    description TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_id       VARCHAR(50) NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id VARCHAR(100) NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id    VARCHAR(50) NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE IF NOT EXISTS user_permissions (
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission_id VARCHAR(100) NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, permission_id)
);

CREATE INDEX IF NOT EXISTS idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_user_permissions_user_id ON user_permissions(user_id);

INSERT INTO roles (id, name, description) VALUES
    ('user', 'User', 'Standard authenticated user'),
    ('admin', 'Administrator', 'Administrative user with full system privileges')
ON CONFLICT (id) DO NOTHING;

INSERT INTO permissions (id, description) VALUES
    ('profile.read', 'Read own user profile'),
    ('profile.write', 'Update own user profile'),
    ('sessions.read', 'List active user sessions'),
    ('sessions.delete', 'Revoke user sessions'),
    ('users.read', 'List and view system users (admin)'),
    ('users.delete', 'Delete users (admin)')
ON CONFLICT (id) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('user', 'profile.read'),
    ('user', 'profile.write'),
    ('user', 'sessions.read'),
    ('user', 'sessions.delete')
ON CONFLICT (role_id, permission_id) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('admin', 'profile.read'),
    ('admin', 'profile.write'),
    ('admin', 'sessions.read'),
    ('admin', 'sessions.delete'),
    ('admin', 'users.read'),
    ('admin', 'users.delete')
ON CONFLICT (role_id, permission_id) DO NOTHING;

INSERT INTO user_roles (user_id, role_id)
SELECT id, 'user' FROM users
ON CONFLICT (user_id, role_id) DO NOTHING;
