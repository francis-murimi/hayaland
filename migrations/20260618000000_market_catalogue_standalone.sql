-- Market catalogue: make resources, needs, and enhancements standalone.
-- Backwards-compatible: adds nullable columns and idempotent indexes.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Resources: become standalone catalogue items.
ALTER TABLE resources
    ALTER COLUMN deal_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS catalog_item_id UUID REFERENCES resources(id),
    ADD COLUMN IF NOT EXISTS metadata JSONB,
    ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS deal_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS platform_hidden BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS platform_featured BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS admin_notes TEXT,
    ADD COLUMN IF NOT EXISTS admin_reviewed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS admin_reviewed_by UUID REFERENCES users(id);

-- Needs: become standalone catalogue items.
ALTER TABLE needs
    ALTER COLUMN deal_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS catalog_item_id UUID REFERENCES needs(id),
    ADD COLUMN IF NOT EXISTS location_geo GEOGRAPHY(POINT),
    ADD COLUMN IF NOT EXISTS metadata JSONB,
    ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS deal_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS platform_hidden BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS platform_featured BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS admin_notes TEXT,
    ADD COLUMN IF NOT EXISTS admin_reviewed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS admin_reviewed_by UUID REFERENCES users(id);

-- Enhancements: become standalone catalogue items.
ALTER TABLE enhancements
    ALTER COLUMN deal_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS catalog_item_id UUID REFERENCES enhancements(id),
    ADD COLUMN IF NOT EXISTS location_geo GEOGRAPHY(POINT),
    ADD COLUMN IF NOT EXISTS metadata JSONB,
    ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS deal_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS platform_hidden BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS platform_featured BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS admin_notes TEXT,
    ADD COLUMN IF NOT EXISTS admin_reviewed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS admin_reviewed_by UUID REFERENCES users(id);

-- Consistency: a row is either a catalogue entry (deal_id IS NULL)
-- or a deal-bound copy of a catalogue entry (catalog_item_id IS NOT NULL),
-- or a deal-bound entry created directly (both deal_id IS NOT NULL AND catalog_item_id IS NULL).
ALTER TABLE resources DROP CONSTRAINT IF EXISTS chk_resource_catalog_or_deal;
ALTER TABLE resources ADD CONSTRAINT chk_resource_catalog_or_deal
    CHECK ((deal_id IS NOT NULL) OR (catalog_item_id IS NULL));

ALTER TABLE needs DROP CONSTRAINT IF EXISTS chk_need_catalog_or_deal;
ALTER TABLE needs ADD CONSTRAINT chk_need_catalog_or_deal
    CHECK ((deal_id IS NOT NULL) OR (catalog_item_id IS NULL));

ALTER TABLE enhancements DROP CONSTRAINT IF EXISTS chk_enhancement_catalog_or_deal;
ALTER TABLE enhancements ADD CONSTRAINT chk_enhancement_catalog_or_deal
    CHECK ((deal_id IS NOT NULL) OR (catalog_item_id IS NULL));

-- Indexes for discovery and search.
CREATE INDEX IF NOT EXISTS idx_resources_party ON resources(supplier_party_id);
CREATE INDEX IF NOT EXISTS idx_resources_type ON resources(resource_type_id);
CREATE INDEX IF NOT EXISTS idx_resources_active ON resources(is_active);
CREATE INDEX IF NOT EXISTS idx_resources_hidden ON resources(platform_hidden);
CREATE INDEX IF NOT EXISTS idx_resources_geo ON resources USING GIST(location_geo);
CREATE INDEX IF NOT EXISTS idx_resources_search ON resources USING GIN (
    (resource_name || ' ' || COALESCE(description, '')) gin_trgm_ops
);

CREATE INDEX IF NOT EXISTS idx_needs_party ON needs(consumer_party_id);
CREATE INDEX IF NOT EXISTS idx_needs_type ON needs(need_category_id);
CREATE INDEX IF NOT EXISTS idx_needs_active ON needs(is_active);
CREATE INDEX IF NOT EXISTS idx_needs_hidden ON needs(platform_hidden);
CREATE INDEX IF NOT EXISTS idx_needs_geo ON needs USING GIST(location_geo);
CREATE INDEX IF NOT EXISTS idx_needs_search ON needs USING GIN (
    (need_description || ' ' || COALESCE(quality_requirements, '')) gin_trgm_ops
);

CREATE INDEX IF NOT EXISTS idx_enhancements_party ON enhancements(enhancer_party_id);
CREATE INDEX IF NOT EXISTS idx_enhancements_type ON enhancements(enhancement_type_id);
CREATE INDEX IF NOT EXISTS idx_enhancements_active ON enhancements(is_active);
CREATE INDEX IF NOT EXISTS idx_enhancements_hidden ON enhancements(platform_hidden);
CREATE INDEX IF NOT EXISTS idx_enhancements_geo ON enhancements USING GIST(location_geo);
CREATE INDEX IF NOT EXISTS idx_enhancements_search ON enhancements USING GIN (
    (enhancement_name || ' ' || COALESCE(description, '')) gin_trgm_ops
);

-- Deal-bound indexes.
CREATE INDEX IF NOT EXISTS idx_resources_deal ON resources(deal_id);
CREATE INDEX IF NOT EXISTS idx_needs_deal ON needs(deal_id);
CREATE INDEX IF NOT EXISTS idx_enhancements_deal ON enhancements(deal_id);
CREATE INDEX IF NOT EXISTS idx_resources_catalog_item ON resources(catalog_item_id);
CREATE INDEX IF NOT EXISTS idx_needs_catalog_item ON needs(catalog_item_id);
CREATE INDEX IF NOT EXISTS idx_enhancements_catalog_item ON enhancements(catalog_item_id);
