-- Store Discord OAuth refresh tokens for periodic guild membership refresh.
-- Refresh tokens don't expire until revoked, allowing background guild list updates.
CREATE TABLE IF NOT EXISTS discord_tokens (
    discord_id          TEXT PRIMARY KEY,
    refresh_token       TEXT NOT NULL,
    guilds_refreshed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
