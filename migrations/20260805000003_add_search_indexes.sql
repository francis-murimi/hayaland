-- Full-text search indexes over parties, catalog, and deals using PostgreSQL tsvector.
-- pg_trgm is already enabled by the market catalogue migration; ensure it exists here as well.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Parties
CREATE INDEX IF NOT EXISTS idx_parties_search_vector
    ON parties USING GIN (
        to_tsvector('english', COALESCE(display_name, '') || ' ' || COALESCE(email, ''))
    );

-- Resources
CREATE INDEX IF NOT EXISTS idx_resources_search_vector
    ON resources USING GIN (
        to_tsvector('english', COALESCE(resource_name, '') || ' ' || COALESCE(description, ''))
    );

-- Needs
CREATE INDEX IF NOT EXISTS idx_needs_search_vector
    ON needs USING GIN (
        to_tsvector('english', COALESCE(need_description, '') || ' ' || COALESCE(quality_requirements, ''))
    );

-- Enhancements
CREATE INDEX IF NOT EXISTS idx_enhancements_search_vector
    ON enhancements USING GIN (
        to_tsvector('english', COALESCE(enhancement_name, '') || ' ' || COALESCE(description, ''))
    );

-- Deals
CREATE INDEX IF NOT EXISTS idx_deals_search_vector
    ON deals USING GIN (
        to_tsvector('english', COALESCE(deal_title, '') || ' ' || COALESCE(deal_description, ''))
    );
