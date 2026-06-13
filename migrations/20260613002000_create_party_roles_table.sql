CREATE TABLE IF NOT EXISTS party_roles (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    role_type TEXT NOT NULL CHECK (role_type IN ('SUPPLIER','CONSUMER','ENHANCER')),
    profile JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (party_id, role_type)
);

CREATE INDEX IF NOT EXISTS idx_party_roles_party ON party_roles(party_id);
CREATE INDEX IF NOT EXISTS idx_party_roles_type ON party_roles(role_type);
