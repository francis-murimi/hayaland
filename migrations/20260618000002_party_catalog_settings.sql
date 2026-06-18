-- Party-level catalogue inquiry controls.

ALTER TABLE parties
    ADD COLUMN IF NOT EXISTS accepts_catalog_inquiries BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS public_contact_email BOOLEAN NOT NULL DEFAULT false;
