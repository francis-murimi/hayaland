-- Category taxonomy for domains, resource types, need types, and enhancement types.
CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    parent_category_id UUID REFERENCES categories(id),
    category_name CITEXT NOT NULL,
    category_code CITEXT NOT NULL UNIQUE,
    description TEXT,
    category_type TEXT NOT NULL CHECK (category_type IN ('DOMAIN','RESOURCE_TYPE','NEED_TYPE','ENHANCEMENT_TYPE','LOCATION','CUSTOM')),
    icon_url TEXT,
    metadata_schema JSONB,
    is_active BOOLEAN NOT NULL DEFAULT true,
    display_order INTEGER NOT NULL DEFAULT 1,
    deal_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_categories_type ON categories(category_type);
CREATE INDEX IF NOT EXISTS idx_categories_parent ON categories(parent_category_id);
