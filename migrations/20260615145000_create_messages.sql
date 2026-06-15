-- Messages store encrypted content and contextual recipient metadata.
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_user_id UUID NOT NULL REFERENCES users(id),
    sender_party_id UUID REFERENCES parties(id),
    recipient_type TEXT NOT NULL
        CHECK (recipient_type IN ('USER','PARTY','PARTY_MEMBERS','DEAL','ROOM','ADMIN_BROADCAST')),
    recipient_user_id UUID REFERENCES users(id),
    recipient_party_id UUID REFERENCES parties(id),
    recipient_deal_id UUID REFERENCES deals(id),
    recipient_room_id UUID REFERENCES chat_rooms(id),
    message_type TEXT NOT NULL
        CHECK (message_type IN ('TEXT','FILE','SYSTEM','ADMIN_BROADCAST')),
    subject TEXT,
    content TEXT NOT NULL,
    content_encryption_key_id UUID REFERENCES encryption_keys(id),
    attachment_urls TEXT[] NOT NULL DEFAULT '{}',
    reply_to_message_id UUID REFERENCES messages(id),
    is_pinned BOOLEAN NOT NULL DEFAULT false,
    pinned_at TIMESTAMPTZ,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    edited_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_message_recipient CHECK (
        (recipient_type = 'USER' AND recipient_user_id IS NOT NULL) OR
        (recipient_type = 'PARTY' AND recipient_party_id IS NOT NULL) OR
        (recipient_type = 'PARTY_MEMBERS' AND sender_party_id IS NOT NULL) OR
        (recipient_type = 'DEAL' AND recipient_deal_id IS NOT NULL) OR
        (recipient_type = 'ROOM' AND recipient_room_id IS NOT NULL) OR
        (recipient_type = 'ADMIN_BROADCAST')
    )
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_created
    ON messages(conversation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_conversation_pinned
    ON messages(conversation_id, is_pinned, pinned_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_sender_user
    ON messages(sender_user_id);
CREATE INDEX IF NOT EXISTS idx_messages_recipient_user
    ON messages(recipient_user_id) WHERE recipient_type = 'USER';
CREATE INDEX IF NOT EXISTS idx_messages_recipient_party
    ON messages(recipient_party_id) WHERE recipient_type = 'PARTY';
CREATE INDEX IF NOT EXISTS idx_messages_recipient_deal
    ON messages(recipient_deal_id) WHERE recipient_type = 'DEAL';
CREATE INDEX IF NOT EXISTS idx_messages_recipient_room
    ON messages(recipient_room_id) WHERE recipient_type = 'ROOM';
CREATE INDEX IF NOT EXISTS idx_messages_reply_to
    ON messages(reply_to_message_id);
