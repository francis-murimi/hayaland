-- Add version to signatures so renegotiated agreements can be re-signed.
ALTER TABLE signatures
    ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

ALTER TABLE signatures
    DROP CONSTRAINT IF EXISTS signatures_agreement_id_party_id_key;

ALTER TABLE signatures
    ADD CONSTRAINT signatures_agreement_party_version_key
        UNIQUE (agreement_id, party_id, version);
