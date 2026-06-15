-- Read receipts for messages.
CREATE TABLE IF NOT EXISTS message_reads (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    party_id UUID REFERENCES parties(id),
    read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_message_reads_message
    ON message_reads(message_id);
CREATE INDEX IF NOT EXISTS idx_message_reads_user
    ON message_reads(user_id);
