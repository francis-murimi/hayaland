-- Core deal aggregate and participations.
CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS deals (
    id UUID PRIMARY KEY,
    deal_reference TEXT NOT NULL UNIQUE,
    deal_title TEXT NOT NULL,
    deal_description TEXT,
    domain_category_id UUID NOT NULL REFERENCES categories(id),
    initiator_party_id UUID NOT NULL REFERENCES parties(id),
    initiator_role TEXT NOT NULL CHECK (initiator_role IN ('SUPPLIER','CONSUMER','ENHANCER')),
    deal_status TEXT NOT NULL DEFAULT 'DRAFT',
    expected_start_date DATE,
    expected_end_date DATE,
    actual_start_date DATE,
    actual_end_date DATE,
    timeline JSONB, -- per-deal timeline: key milestones/dates negotiated by parties
    location_geo GEOGRAPHY(POINT),
    location_address JSONB,
    total_deal_value DECIMAL, -- in platform points
    currency TEXT DEFAULT 'POINTS',
    platform_fee_percentage DECIMAL NOT NULL DEFAULT 0, -- set per deal
    platform_fee_amount DECIMAL NOT NULL DEFAULT 0, -- in platform points
    win_win_win_validated BOOLEAN NOT NULL DEFAULT false,
    validation_checked_at TIMESTAMPTZ,
    validation_score DECIMAL,
    validation_result JSONB,
    is_public BOOLEAN NOT NULL DEFAULT false, -- only affects match/discovery metadata, never exposes deal details
    current_state_entered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS deal_participations (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    role TEXT NOT NULL CHECK (role IN ('SUPPLIER','CONSUMER','ENHANCER')),
    participation_status TEXT NOT NULL DEFAULT 'INVITED' CHECK (participation_status IN ('INVITED','PENDING','ACCEPTED','DECLINED','WITHDRAWN')),
    is_initiator BOOLEAN NOT NULL DEFAULT false,
    value_share_percentage DECIMAL,
    value_share_amount DECIMAL,
    invited_at TIMESTAMPTZ,
    responded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (deal_id, role),
    UNIQUE (deal_id, party_id)
);

CREATE TABLE IF NOT EXISTS deal_history (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    actor_party_id UUID REFERENCES parties(id),
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_deals_status ON deals(deal_status);
CREATE INDEX IF NOT EXISTS idx_deals_initiator ON deals(initiator_party_id);
CREATE INDEX IF NOT EXISTS idx_deals_domain ON deals(domain_category_id);
CREATE INDEX IF NOT EXISTS idx_deals_geo ON deals USING GIST(location_geo);
CREATE INDEX IF NOT EXISTS idx_participations_party ON deal_participations(party_id);
CREATE INDEX IF NOT EXISTS idx_participations_deal ON deal_participations(deal_id);
CREATE INDEX IF NOT EXISTS idx_deal_history_deal ON deal_history(deal_id, created_at DESC);
