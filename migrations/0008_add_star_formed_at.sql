ALTER TABLE andromeda_stars
ADD COLUMN formed_at TIMESTAMPTZ;

UPDATE andromeda_stars
SET formed_at = updated_at
WHERE star_status = 'published';

ALTER TABLE andromeda_stars
ADD CONSTRAINT published_star_formed_at
CHECK (star_status <> 'published' OR formed_at IS NOT NULL);
