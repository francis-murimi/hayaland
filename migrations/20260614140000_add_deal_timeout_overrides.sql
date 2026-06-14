-- Per-deal timeout overrides.
ALTER TABLE deals
    ADD COLUMN IF NOT EXISTS timeout_overrides JSONB;

-- Index to make timeout scanning fast.
CREATE INDEX IF NOT EXISTS idx_deals_status_entered_at
    ON deals(deal_status, current_state_entered_at)
    WHERE deal_status NOT IN ('COMPLETED', 'CANCELLED', 'EXPIRED');
