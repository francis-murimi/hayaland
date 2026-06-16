-- Trust-score counters and supporting indexes for the trust-score module.

ALTER TABLE trust_scores
    ADD COLUMN IF NOT EXISTS timeouts_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS no_shows_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_completed_value DECIMAL NOT NULL DEFAULT 0;

-- Use floating-point scores and 64-bit counters for the trust-score service.
ALTER TABLE trust_scores
    ALTER COLUMN overall_score TYPE DOUBLE PRECISION USING overall_score::double precision,
    ALTER COLUMN as_supplier_score TYPE DOUBLE PRECISION USING as_supplier_score::double precision,
    ALTER COLUMN as_consumer_score TYPE DOUBLE PRECISION USING as_consumer_score::double precision,
    ALTER COLUMN as_enhancer_score TYPE DOUBLE PRECISION USING as_enhancer_score::double precision,
    ALTER COLUMN deals_completed_count TYPE BIGINT USING deals_completed_count::bigint,
    ALTER COLUMN deals_cancelled_count TYPE BIGINT USING deals_cancelled_count::bigint,
    ALTER COLUMN deals_disputed_count TYPE BIGINT USING deals_disputed_count::bigint,
    ALTER COLUMN timeouts_count TYPE BIGINT USING timeouts_count::bigint,
    ALTER COLUMN no_shows_count TYPE BIGINT USING no_shows_count::bigint,
    ALTER COLUMN total_completed_value TYPE DOUBLE PRECISION USING total_completed_value::double precision,
    ALTER COLUMN average_response_hours TYPE DOUBLE PRECISION USING average_response_hours::double precision,
    ALTER COLUMN profile_completeness TYPE DOUBLE PRECISION USING profile_completeness::double precision,
    ALTER COLUMN longevity_days TYPE BIGINT USING longevity_days::bigint;

CREATE INDEX IF NOT EXISTS idx_deal_participations_party_role
    ON deal_participations(party_id, role, deal_id);

CREATE INDEX IF NOT EXISTS idx_reviews_reviewed_party_role_created
    ON reviews(reviewed_party_id, reviewed_role, created_at);

CREATE INDEX IF NOT EXISTS idx_messages_recipient_party_created
    ON messages(recipient_party_id, created_at)
    WHERE recipient_party_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_message_reads_message
    ON message_reads(message_id);
