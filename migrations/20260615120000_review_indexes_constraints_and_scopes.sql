-- Enforce one review per (deal, reviewer, reviewed) tuple.
CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_unique_pair
    ON reviews(deal_id, reviewer_party_id, reviewed_party_id);

-- Lookup public reviews for a party.
CREATE INDEX IF NOT EXISTS idx_reviews_reviewed_party
    ON reviews(reviewed_party_id, is_public, created_at DESC);

-- Lookup reviews authored by a party.
CREATE INDEX IF NOT EXISTS idx_reviews_reviewer_party
    ON reviews(reviewer_party_id, created_at DESC);

-- Grant regular users the review scopes.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['reviews:read', 'reviews:write'])
)
WHERE name = 'user';

-- Grant admins review management scope.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:reviews'])
)
WHERE name = 'admin';
