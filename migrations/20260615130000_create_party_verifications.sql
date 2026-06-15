CREATE TABLE IF NOT EXISTS party_verifications (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
    requested_by_user_id UUID NOT NULL REFERENCES users(id),
    reviewed_by_user_id UUID REFERENCES users(id),
    verification_type TEXT NOT NULL
        CHECK (verification_type IN (
            'EMAIL', 'PHONE', 'GOVERNMENT_ID', 'BUSINESS_REGISTRATION',
            'BANK_ACCOUNT', 'PROFESSIONAL_CERTIFICATION', 'VIDEO_INTERVIEW'
        )),
    status TEXT NOT NULL DEFAULT 'PENDING'
        CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'EXPIRED', 'REVOKED')),
    points INTEGER NOT NULL,
    evidence_urls TEXT[] NOT NULL DEFAULT '{}',
    provider_reference TEXT,
    provider_payload JSONB,
    rejection_reason TEXT,
    review_notes TEXT,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_party_verifications_unique_active
    ON party_verifications(party_id, verification_type)
    WHERE status IN ('PENDING', 'APPROVED');

CREATE INDEX IF NOT EXISTS idx_party_verifications_party
    ON party_verifications(party_id, status, verification_type);

CREATE INDEX IF NOT EXISTS idx_party_verifications_status
    ON party_verifications(status, requested_at);

CREATE INDEX IF NOT EXISTS idx_party_verifications_type
    ON party_verifications(verification_type, status);

-- Scope grants for the verifications feature.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['verifications:read', 'verifications:write'])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:verifications'])
)
WHERE name = 'admin';
