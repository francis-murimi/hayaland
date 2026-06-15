-- Chatroom memberships link users or parties to chatrooms.
CREATE TABLE IF NOT EXISTS chat_room_memberships (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES chat_rooms(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    party_id UUID REFERENCES parties(id) ON DELETE CASCADE,
    member_role TEXT NOT NULL DEFAULT 'MEMBER' CHECK (member_role IN ('MEMBER','MODERATOR')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (room_id, user_id),
    UNIQUE (room_id, party_id),
    CONSTRAINT chk_membership_actor CHECK (
        (user_id IS NOT NULL AND party_id IS NULL) OR
        (user_id IS NULL AND party_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_chat_room_memberships_room
    ON chat_room_memberships(room_id);
CREATE INDEX IF NOT EXISTS idx_chat_room_memberships_user
    ON chat_room_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_chat_room_memberships_party
    ON chat_room_memberships(party_id);
