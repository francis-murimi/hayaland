-- Trigger to enforce the per-conversation pinned message limit.
-- When a message is pinned, older pinned messages in the same conversation
-- are automatically unpinned so that at most MAX_PINNED messages remain pinned.
CREATE OR REPLACE FUNCTION enforce_message_pin_limit()
RETURNS TRIGGER AS $$
DECLARE
    max_pinned INT := COALESCE(current_setting('app.messages.max_pinned_per_conversation', true)::INT, 5);
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.is_pinned = true AND (OLD.is_pinned = false OR OLD.is_pinned IS NULL) THEN
        UPDATE messages
        SET is_pinned = false, pinned_at = NULL
        WHERE id IN (
            SELECT id FROM messages
            WHERE conversation_id = NEW.conversation_id
              AND is_pinned = true
              AND id != NEW.id
            ORDER BY pinned_at DESC
            OFFSET max_pinned - 1
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_enforce_message_pin_limit ON messages;
CREATE TRIGGER trg_enforce_message_pin_limit
    AFTER UPDATE OF is_pinned ON messages
    FOR EACH ROW
    EXECUTE FUNCTION enforce_message_pin_limit();
