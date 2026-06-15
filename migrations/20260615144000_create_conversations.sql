-- Conversations group messages by messaging context.
CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY,
    conversation_type TEXT NOT NULL
        CHECK (conversation_type IN ('DIRECT_USER','DIRECT_PARTY','PARTY_MEMBERS','DEAL','ROOM','ADMIN_BROADCAST')),
    user_a_id UUID REFERENCES users(id),
    user_b_id UUID REFERENCES users(id),
    party_a_id UUID REFERENCES parties(id),
    party_b_id UUID REFERENCES parties(id),
    party_id UUID REFERENCES parties(id),
    deal_id UUID REFERENCES deals(id),
    room_id UUID REFERENCES chat_rooms(id),
    title TEXT,
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_direct_user_members CHECK (
        conversation_type != 'DIRECT_USER' OR (user_a_id IS NOT NULL AND user_b_id IS NOT NULL)
    ),
    CONSTRAINT chk_direct_party_members CHECK (
        conversation_type != 'DIRECT_PARTY' OR (party_a_id IS NOT NULL AND party_b_id IS NOT NULL)
    ),
    CONSTRAINT chk_party_members_context CHECK (
        conversation_type != 'PARTY_MEMBERS' OR party_id IS NOT NULL
    ),
    CONSTRAINT chk_deal_context CHECK (
        conversation_type != 'DEAL' OR deal_id IS NOT NULL
    ),
    CONSTRAINT chk_room_context CHECK (
        conversation_type != 'ROOM' OR room_id IS NOT NULL
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_direct_user
    ON conversations(user_a_id, user_b_id) WHERE conversation_type = 'DIRECT_USER';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_direct_party
    ON conversations(party_a_id, party_b_id) WHERE conversation_type = 'DIRECT_PARTY';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_party_members
    ON conversations(party_id) WHERE conversation_type = 'PARTY_MEMBERS';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_deal
    ON conversations(deal_id) WHERE conversation_type = 'DEAL';
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_room
    ON conversations(room_id) WHERE conversation_type = 'ROOM';
