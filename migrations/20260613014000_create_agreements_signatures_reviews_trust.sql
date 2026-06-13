-- Agreements, signatures, reviews, and trust scores.
CREATE TABLE IF NOT EXISTS agreements (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL UNIQUE REFERENCES deals(id) ON DELETE CASCADE,
    agreement_status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (agreement_status IN ('DRAFT','PENDING_SIGNATURES','SIGNED','EXECUTED','TERMINATED')),
    agreement_text TEXT NOT NULL,
    governing_law TEXT,
    dispute_resolution TEXT,
    effective_date DATE,
    termination_date DATE,
    auto_renew BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1,
    digital_signature_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    executed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS signatures (
    id UUID PRIMARY KEY,
    agreement_id UUID NOT NULL REFERENCES agreements(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    signed_by_user_id UUID NOT NULL REFERENCES users(id),
    signature_type TEXT NOT NULL,
    signature_data TEXT NOT NULL,
    ip_address TEXT,
    signed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, party_id)
);

CREATE TABLE IF NOT EXISTS reviews (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    reviewer_party_id UUID NOT NULL REFERENCES parties(id),
    reviewed_party_id UUID NOT NULL REFERENCES parties(id),
    reviewed_role TEXT NOT NULL,
    overall_rating INTEGER NOT NULL CHECK (overall_rating BETWEEN 1 AND 5),
    communication_rating INTEGER CHECK (communication_rating BETWEEN 1 AND 5),
    reliability_rating INTEGER CHECK (reliability_rating BETWEEN 1 AND 5),
    quality_rating INTEGER CHECK (quality_rating BETWEEN 1 AND 5),
    timeliness_rating INTEGER CHECK (timeliness_rating BETWEEN 1 AND 5),
    review_text TEXT,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    is_public BOOLEAN NOT NULL DEFAULT true,
    platform_response TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS trust_scores (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    overall_score DECIMAL NOT NULL DEFAULT 0,
    as_supplier_score DECIMAL,
    as_consumer_score DECIMAL,
    as_enhancer_score DECIMAL,
    deals_completed_count INTEGER NOT NULL DEFAULT 0,
    deals_cancelled_count INTEGER NOT NULL DEFAULT 0,
    deals_disputed_count INTEGER NOT NULL DEFAULT 0,
    average_response_hours DECIMAL,
    profile_completeness DECIMAL NOT NULL DEFAULT 0,
    verification_level INTEGER NOT NULL DEFAULT 0,
    longevity_days INTEGER NOT NULL DEFAULT 0,
    calculation_formula JSONB NOT NULL DEFAULT '{}',
    last_calculated_at TIMESTAMPTZ,
    next_calculation_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_signatures_agreement ON signatures(agreement_id);
CREATE INDEX IF NOT EXISTS idx_reviews_deal ON reviews(deal_id);
CREATE INDEX IF NOT EXISTS idx_trust_scores_party ON trust_scores(party_id);
