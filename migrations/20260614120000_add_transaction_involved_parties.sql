-- Add involved-party tracking to transactions so pending approval workflows
-- can determine which parties need to approve (especially 3-party escrow releases).
ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS involved_party_ids UUID[] NOT NULL DEFAULT ARRAY[]::UUID[];

CREATE INDEX IF NOT EXISTS idx_transactions_involved_parties
    ON transactions USING GIN (involved_party_ids);
