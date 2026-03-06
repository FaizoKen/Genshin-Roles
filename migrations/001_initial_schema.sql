CREATE TABLE IF NOT EXISTS role_links (
    id              BIGSERIAL PRIMARY KEY,
    guild_id        TEXT NOT NULL,
    role_id         TEXT NOT NULL,
    api_token       TEXT NOT NULL,
    conditions      JSONB NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (guild_id, role_id)
);

CREATE TABLE IF NOT EXISTS linked_accounts (
    id              BIGSERIAL PRIMARY KEY,
    discord_id      TEXT NOT NULL UNIQUE,
    uid             TEXT NOT NULL UNIQUE,
    linked_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS player_cache (
    uid             TEXT PRIMARY KEY,
    player_info     JSONB NOT NULL,
    region          TEXT,
    enka_ttl        INTEGER NOT NULL DEFAULT 60,
    fetched_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    next_fetch_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    fetch_failures  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_player_cache_next_fetch ON player_cache (next_fetch_at ASC);

CREATE TABLE IF NOT EXISTS verification_sessions (
    id              BIGSERIAL PRIMARY KEY,
    discord_id      TEXT NOT NULL,
    uid             TEXT NOT NULL,
    code            TEXT NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    attempts        INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_verification_discord ON verification_sessions (discord_id);

CREATE TABLE IF NOT EXISTS role_assignments (
    guild_id        TEXT NOT NULL,
    role_id         TEXT NOT NULL,
    discord_id      TEXT NOT NULL,
    assigned_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, role_id, discord_id),
    FOREIGN KEY (guild_id, role_id) REFERENCES role_links (guild_id, role_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS oauth_states (
    state           TEXT PRIMARY KEY,
    redirect_data   JSONB,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
