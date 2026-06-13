CREATE TABLE IF NOT EXISTS parties (
    id UUID PRIMARY KEY,
    party_type TEXT NOT NULL CHECK (party_type IN ('INDIVIDUAL','ORGANIZATION','PARTY_GROUP')),
    display_name CITEXT NOT NULL,
    email CITEXT NOT NULL UNIQUE,
    phone TEXT,
    tax_id TEXT,
    verification_status TEXT NOT NULL DEFAULT 'UNVERIFIED' CHECK (verification_status IN ('UNVERIFIED','PENDING','VERIFIED','REJECTED')),
    primary_domain_id UUID,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    service_radius_km DOUBLE PRECISION,
    trust_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_deals_completed INTEGER NOT NULL DEFAULT 0,
    total_deals_initiated INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_parties_email ON parties(email);
CREATE INDEX IF NOT EXISTS idx_parties_type ON parties(party_type);
CREATE INDEX IF NOT EXISTS idx_parties_primary_domain ON parties(primary_domain_id);
CREATE INDEX IF NOT EXISTS idx_parties_location ON parties(latitude, longitude);
