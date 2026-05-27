-- Add a stable URL-safe `key` to recipes and meals so they can be addressed
-- by name-derived key instead of integer id (`/recipes/black-beans`).
--
-- Defaulting to '' is a workaround: SQLite can't promote a nullable column to
-- NOT NULL without a table rebuild, so we add the column NOT NULL up front
-- and immediately overwrite every existing row in the same migration. The
-- UNIQUE INDEX at the end fails loudly if any row is still '' (i.e. backfill
-- missed it) or if two backfilled keys collide.
--
-- The SQL keyify here is a deliberate approximation of `types::keyify`. It
-- handles English names with common punctuation, which covers everything
-- currently in the DB; new rows go through the richer Rust implementation.

ALTER TABLE recipes ADD COLUMN key TEXT NOT NULL DEFAULT '';
ALTER TABLE meals   ADD COLUMN key TEXT NOT NULL DEFAULT '';

UPDATE recipes
   SET key = LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
             TRIM(name),
             '''', ''), '"', ''), ',', ''), '.', ''), '/', '-'), '&', 'and'), ' ', '-'));

UPDATE meals
   SET key = LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
             TRIM(name),
             '''', ''), '"', ''), ',', ''), '.', ''), '/', '-'), '&', 'and'), ' ', '-'));

CREATE UNIQUE INDEX recipes_key_unique ON recipes(key);
CREATE UNIQUE INDEX meals_key_unique   ON meals(key);
