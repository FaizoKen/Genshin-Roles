-- Combine tower_floor and tower_level into a single abyss_progress column.
-- Value = floor * 10 + chamber (e.g. floor 12 chamber 3 = 123).
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS abyss_progress INTEGER NOT NULL DEFAULT 0;

-- Backfill from player_info JSONB (idempotent, works after columns are dropped)
UPDATE player_cache SET
    abyss_progress = COALESCE((player_info->>'towerFloorIndex')::int, 0) * 10
                   + COALESCE((player_info->>'towerLevelIndex')::int, 0)
WHERE abyss_progress = 0 AND player_info != '{}'::jsonb;

ALTER TABLE player_cache DROP COLUMN IF EXISTS tower_floor;
ALTER TABLE player_cache DROP COLUMN IF EXISTS tower_level;
