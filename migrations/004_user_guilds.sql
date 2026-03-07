-- Track which guilds each verified user belongs to (captured via Discord OAuth guilds scope).
-- Used to scope sync operations: only PUT users who are actually in the guild.
CREATE TABLE IF NOT EXISTS user_guilds (
    discord_id  TEXT NOT NULL,
    guild_id    TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (discord_id, guild_id)
);

CREATE INDEX IF NOT EXISTS idx_user_guilds_guild ON user_guilds (guild_id);
