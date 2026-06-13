-- Child deal entities: resources, needs, enhancements.
CREATE TABLE IF NOT EXISTS resources (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    supplier_party_id UUID NOT NULL REFERENCES parties(id),
    resource_type_id UUID NOT NULL REFERENCES categories(id),
    resource_name TEXT NOT NULL,
    description TEXT,
    quantity DECIMAL NOT NULL,
    quantity_unit TEXT NOT NULL,
    condition TEXT,
    location_geo GEOGRAPHY(POINT),
    availability_start DATE,
    availability_end DATE,
    document_urls TEXT[],
    opportunity_cost DECIMAL, -- in platform points
    verified_by_platform BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS needs (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    consumer_party_id UUID NOT NULL REFERENCES parties(id),
    need_category_id UUID NOT NULL REFERENCES categories(id),
    need_description TEXT NOT NULL,
    required_quantity DECIMAL NOT NULL,
    quantity_unit TEXT NOT NULL,
    quality_requirements TEXT,
    required_by_date DATE,
    max_budget DECIMAL, -- in platform points
    budget_currency TEXT DEFAULT 'POINTS',
    estimated_fulfillment_value DECIMAL, -- in platform points
    acceptable_variants TEXT,
    priority TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS enhancements (
    id UUID PRIMARY KEY,
    deal_id UUID NOT NULL REFERENCES deals(id) ON DELETE CASCADE,
    enhancer_party_id UUID NOT NULL REFERENCES parties(id),
    enhancement_type_id UUID NOT NULL REFERENCES categories(id),
    enhancement_name TEXT NOT NULL,
    description TEXT,
    input_quantity DECIMAL,
    quantity_unit TEXT,
    estimated_input_cost DECIMAL, -- in platform points
    service_duration_hours DECIMAL,
    estimated_completion_days INTEGER,
    deliverables TEXT,
    prerequisites TEXT,
    is_complete BOOLEAN NOT NULL DEFAULT false,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_resources_deal ON resources(deal_id);
CREATE INDEX IF NOT EXISTS idx_needs_deal ON needs(deal_id);
CREATE INDEX IF NOT EXISTS idx_enhancements_deal ON enhancements(deal_id);
