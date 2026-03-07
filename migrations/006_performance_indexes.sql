-- Index for guild refresh worker's pick_next_user() ORDER BY guilds_refreshed_at.
-- Without this, the query does a sequential scan at scale (1000+ users with tokens).
CREATE INDEX IF NOT EXISTS idx_discord_tokens_guilds_refreshed
ON discord_tokens (guilds_refreshed_at ASC);
