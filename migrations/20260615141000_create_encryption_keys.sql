-- Encryption key registry for AES-256-GCM message encryption and key rotation.
CREATE TABLE IF NOT EXISTS encryption_keys (
    id UUID PRIMARY KEY,
    key_name TEXT NOT NULL UNIQUE,
    key_bytes TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
