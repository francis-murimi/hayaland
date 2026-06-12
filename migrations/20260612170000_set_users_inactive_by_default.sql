-- New registrations must verify their email before becoming active.
ALTER TABLE users ALTER COLUMN is_active SET DEFAULT false;
