-- Centralize per-guild settings (currently just Player List Access).
-- Previously stored per-row on role_links, which allowed drift between
-- multiple role links for the same guild. One row per guild, one source
-- of truth.
CREATE TABLE IF NOT EXISTS guild_settings (
    guild_id        TEXT PRIMARY KEY,
    view_permission TEXT NOT NULL DEFAULT 'members'
                    CHECK (view_permission IN ('members', 'managers')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Backfill from existing role_links using the same "most permissive wins"
-- semantics that runtime resolution currently applies, so behavior is
-- preserved on first deploy.
INSERT INTO guild_settings (guild_id, view_permission)
SELECT guild_id,
       CASE WHEN BOOL_OR(view_permission = 'members')
            THEN 'members' ELSE 'managers' END
FROM role_links
GROUP BY guild_id
ON CONFLICT (guild_id) DO NOTHING;

-- The per-row column is now redundant.
ALTER TABLE role_links DROP COLUMN IF EXISTS view_permission;
