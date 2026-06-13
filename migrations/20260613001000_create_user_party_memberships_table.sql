CREATE TABLE IF NOT EXISTS user_party_memberships (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    member_role TEXT NOT NULL DEFAULT 'MEMBER' CHECK (member_role IN ('OWNER','ADMIN','MEMBER','OBSERVER')),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, party_id)
);

CREATE INDEX IF NOT EXISTS idx_user_party_memberships_user ON user_party_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_user_party_memberships_party ON user_party_memberships(party_id);
