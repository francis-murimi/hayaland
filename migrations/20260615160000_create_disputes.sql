CREATE TABLE IF NOT EXISTS disputes (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    raised_by_party_id UUID NOT NULL REFERENCES parties(id),
    raised_by_user_id UUID NOT NULL REFERENCES users(id),
    against_party_id UUID REFERENCES parties(id),
    dispute_type TEXT NOT NULL
        CHECK (dispute_type IN (
            'NON_PAYMENT','NON_DELIVERY','QUALITY_ISSUE','BREACH_OF_TERMS',
            'COMMUNICATION','SCOPE_DISAGREEMENT','DELIVERY_DELAY','FORCE_MAJEURE',
            'FRAUD','OTHER'
        )),
    dispute_status TEXT NOT NULL DEFAULT 'OPEN'
        CHECK (dispute_status IN ('OPEN','UNDER_REVIEW','MEDIATION','ESCALATED','RESOLVED','REJECTED')),
    resolution_type TEXT
        CHECK (resolution_type IN ('AMICABLE','MEDIATED','ARBITRATED','WITHDRAWN')),
    resolution_outcome TEXT
        CHECK (resolution_outcome IN ('IN_FAVOR_OF_RAISED','IN_FAVOR_OF_AGAINST','SPLIT','DISMISSED')),
    severity TEXT
        CHECK (severity IN ('LOW','MEDIUM','HIGH')),
    description TEXT NOT NULL,
    evidence_urls TEXT[] NOT NULL DEFAULT '{}',
    admin_notes TEXT,
    resolution_notes TEXT,
    resolved_by_user_id UUID REFERENCES users(id),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_disputes_unique_open
    ON disputes(deal_id, raised_by_party_id)
    WHERE dispute_status IN ('OPEN','UNDER_REVIEW','MEDIATION','ESCALATED');

CREATE INDEX IF NOT EXISTS idx_disputes_deal ON disputes(deal_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_disputes_raised_by ON disputes(raised_by_party_id, dispute_status);
CREATE INDEX IF NOT EXISTS idx_disputes_against ON disputes(against_party_id, dispute_status);
CREATE INDEX IF NOT EXISTS idx_disputes_status ON disputes(dispute_status, created_at DESC);

CREATE TABLE IF NOT EXISTS dispute_responses (
    id UUID PRIMARY KEY,
    dispute_id UUID NOT NULL REFERENCES disputes(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    user_id UUID NOT NULL REFERENCES users(id),
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dispute_responses_dispute
    ON dispute_responses(dispute_id, created_at ASC);

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['disputes:read', 'disputes:write'])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:disputes'])
)
WHERE name = 'admin';
