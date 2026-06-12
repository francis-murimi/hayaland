CREATE TABLE IF NOT EXISTS role_definitions (
    name TEXT PRIMARY KEY,
    scopes TEXT[] NOT NULL,
    is_builtin BOOLEAN NOT NULL DEFAULT false
);

INSERT INTO role_definitions (name, scopes, is_builtin)
VALUES
    ('user', ARRAY['users:read', 'users:write'], true),
    ('admin', ARRAY['users:read', 'users:write', 'users:admin', 'users:delete'], true)
ON CONFLICT (name) DO NOTHING;
