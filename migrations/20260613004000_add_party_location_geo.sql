-- Enable PostGIS for GEOGRAPHY columns and GIST spatial indexes.
CREATE EXTENSION IF NOT EXISTS postgis;

-- Add a geography column for performant radius and proximity queries.
ALTER TABLE parties
    ADD COLUMN IF NOT EXISTS location_geo GEOGRAPHY(POINT, 4326);

-- Backfill the geography column from the existing plain lat/long columns.
UPDATE parties
SET location_geo = ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography
WHERE latitude IS NOT NULL
  AND longitude IS NOT NULL
  AND location_geo IS NULL;

-- Spatial index for fast ST_DWithin / ST_Distance queries.
CREATE INDEX IF NOT EXISTS idx_parties_geo ON parties USING GIST(location_geo);
