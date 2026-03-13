ALTER TABLE player_cache ADD COLUMN IF NOT EXISTS abyss_stars INTEGER NOT NULL DEFAULT 0;

UPDATE player_cache SET
    abyss_stars = COALESCE((player_info->>'towerStarIndex')::int, 0)
WHERE abyss_stars = 0 AND player_info != '{}'::jsonb;
