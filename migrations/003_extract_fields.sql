-- Extract commonly-queried fields from player_info JSONB into indexed columns.
-- This allows SQL-side condition filtering instead of streaming all rows to Rust.

ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS level INTEGER NOT NULL DEFAULT 0;
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS world_level INTEGER NOT NULL DEFAULT 0;
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS achievements INTEGER NOT NULL DEFAULT 0;
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS tower_floor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS tower_level INTEGER NOT NULL DEFAULT 0;
ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS fetter_count INTEGER NOT NULL DEFAULT 0;

-- Backfill from existing JSONB data
UPDATE player_cache SET
    level = COALESCE((player_info->>'level')::int, 0),
    world_level = COALESCE((player_info->>'worldLevel')::int, 0),
    achievements = COALESCE((player_info->>'finishAchievementNum')::int, 0),
    tower_floor = COALESCE((player_info->>'towerFloorIndex')::int, 0),
    tower_level = COALESCE((player_info->>'towerLevelIndex')::int, 0),
    fetter_count = COALESCE((player_info->>'fetterCount')::int, 0)
WHERE level = 0 AND player_info != '{}'::jsonb;

-- Indexes for SQL-side condition filtering
CREATE INDEX IF NOT EXISTS idx_player_cache_level ON player_cache (level);
CREATE INDEX IF NOT EXISTS idx_player_cache_region ON player_cache (region);
