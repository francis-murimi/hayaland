-- Extend match_suggestions with explainability, counter-propose support, and admin audit fields.
ALTER TABLE match_suggestions
    ADD COLUMN IF NOT EXISTS score_breakdown JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS counter_notes TEXT,
    ADD COLUMN IF NOT EXISTS responded_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Include COUNTER_PROPOSED in the allowed lifecycle statuses.
ALTER TABLE match_suggestions
    DROP CONSTRAINT IF EXISTS match_suggestions_match_status_check;
ALTER TABLE match_suggestions
    ADD CONSTRAINT match_suggestions_match_status_check
        CHECK (match_status IN ('PENDING','ACCEPTED','DECLINED','COUNTER_PROPOSED','EXPIRED','CONVERTED_TO_DEAL'));

-- Composite indexes for listing pending matches for a participant party.
CREATE INDEX IF NOT EXISTS idx_match_suggestions_party_status_supplier
    ON match_suggestions(supplier_party_id, match_status, match_score DESC);
CREATE INDEX IF NOT EXISTS idx_match_suggestions_party_status_consumer
    ON match_suggestions(consumer_party_id, match_status, match_score DESC);
CREATE INDEX IF NOT EXISTS idx_match_suggestions_party_status_enhancer
    ON match_suggestions(enhancer_party_id, match_status, match_score DESC);

-- Score-based discovery index for pending suggestions.
CREATE INDEX IF NOT EXISTS idx_match_suggestions_score
    ON match_suggestions(match_score DESC)
    WHERE match_status = 'PENDING';

-- Audit log for admin mutations on match suggestions.
CREATE TABLE IF NOT EXISTS match_suggestion_audit_log (
    id UUID PRIMARY KEY,
    admin_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action_type TEXT NOT NULL,
    match_suggestion_id UUID REFERENCES match_suggestions(id) ON DELETE SET NULL,
    party_id UUID REFERENCES parties(id) ON DELETE SET NULL,
    before_snapshot JSONB,
    after_snapshot JSONB,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_match_suggestion_audit_log_match
    ON match_suggestion_audit_log(match_suggestion_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_match_suggestion_audit_log_admin
    ON match_suggestion_audit_log(admin_user_id, created_at DESC);
