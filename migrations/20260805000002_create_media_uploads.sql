-- Media uploads: tracks uploaded files stored by the configured MediaStorage port.
CREATE TABLE IF NOT EXISTS media_uploads (
    id UUID PRIMARY KEY,
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    owner_party_id UUID REFERENCES parties(id) ON DELETE SET NULL,
    purpose TEXT NOT NULL CHECK (purpose IN (
        'MESSAGE_ATTACHMENT', 'DISPUTE_EVIDENCE', 'VERIFICATION_EVIDENCE',
        'AGREEMENT_DOCUMENT', 'OTHER'
    )),
    related_entity_type TEXT,
    related_entity_id UUID,
    original_filename TEXT NOT NULL,
    stored_filename TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256 TEXT NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_media_uploads_owner_user ON media_uploads(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_media_uploads_owner_party ON media_uploads(owner_party_id);
CREATE INDEX IF NOT EXISTS idx_media_uploads_related_entity ON media_uploads(related_entity_type, related_entity_id)
    WHERE related_entity_type IS NOT NULL AND related_entity_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_uploads_created_at ON media_uploads(created_at DESC);
