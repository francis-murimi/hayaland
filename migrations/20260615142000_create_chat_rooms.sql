-- Platform-wide chatrooms.
CREATE TABLE IF NOT EXISTS chat_rooms (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    room_type TEXT NOT NULL CHECK (room_type IN ('PUBLIC','PRIVATE')),
    created_by_user_id UUID NOT NULL REFERENCES users(id),
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_chat_rooms_type
    ON chat_rooms(room_type) WHERE is_deleted = false;
CREATE INDEX IF NOT EXISTS idx_chat_rooms_deleted
    ON chat_rooms(is_deleted);
