-- Negotiation and execution child entities.
CREATE TABLE IF NOT EXISTS terms (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    proposed_by_party_id UUID NOT NULL REFERENCES parties(id),
    term_type TEXT NOT NULL,
    term_name TEXT NOT NULL,
    description TEXT NOT NULL,
    negotiation_status TEXT NOT NULL DEFAULT 'PROPOSED' CHECK (negotiation_status IN ('PROPOSED','ACCEPTED','REJECTED','COUNTERED','WITHDRAWN')),
    parent_term_id UUID REFERENCES terms(id),
    version INTEGER NOT NULL DEFAULT 1,
    proposed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    is_mandatory BOOLEAN NOT NULL DEFAULT false,
    resolution TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS value_distributions (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL UNIQUE REFERENCES deals(id) ON DELETE CASCADE,
    total_value DECIMAL NOT NULL, -- in platform points
    currency TEXT NOT NULL DEFAULT 'POINTS',
    distribution_model TEXT NOT NULL,
    supplier_share_percentage DECIMAL NOT NULL,
    supplier_share_amount DECIMAL NOT NULL,
    consumer_cost_percentage DECIMAL NOT NULL,
    consumer_cost_amount DECIMAL NOT NULL,
    enhancer_share_percentage DECIMAL NOT NULL,
    enhancer_share_amount DECIMAL NOT NULL,
    platform_fee_percentage DECIMAL NOT NULL,
    platform_fee_amount DECIMAL NOT NULL,
    payment_schedule JSONB NOT NULL DEFAULT '[]',
    win_win_win_score DECIMAL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS milestones (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    milestone_name TEXT NOT NULL,
    description TEXT,
    assigned_to_party_id UUID REFERENCES parties(id),
    due_date DATE,
    completion_criteria TEXT NOT NULL,
    milestone_status TEXT NOT NULL DEFAULT 'PENDING' CHECK (milestone_status IN ('PENDING','IN_PROGRESS','COMPLETED','VERIFIED','MISSED')),
    completion_percentage DECIMAL NOT NULL DEFAULT 0,
    payment_trigger_amount DECIMAL, -- in platform points
    completed_at TIMESTAMPTZ,
    verified_by_party_id UUID REFERENCES parties(id),
    display_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_terms_deal ON terms(deal_id);
CREATE INDEX IF NOT EXISTS idx_milestones_deal ON milestones(deal_id);
