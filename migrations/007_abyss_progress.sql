-- Combine tower_floor and tower_level into a single abyss_progress column.
-- Value = floor * 10 + chamber (e.g. floor 12 chamber 3 = 123).
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS abyss_progress INTEGER NOT NULL DEFAULT 0;

UPDATE player_cache SET abyss_progress = tower_floor * 10 + tower_level;

ALTER TABLE player_cache DROP COLUMN IF EXISTS tower_floor;
ALTER TABLE player_cache DROP COLUMN IF EXISTS tower_level;
