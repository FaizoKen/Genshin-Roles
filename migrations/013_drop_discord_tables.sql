-- Discord-related tables now live exclusively in the Auth Gateway database.
-- Plugin sync code reads guild membership via /auth/internal/* HTTP endpoints
-- instead of JOINing against a local mirror.
--
-- Run `cargo run --bin migrate_to_gateway` BEFORE this migration ships in
-- production so any rows that exist only in this DB get copied over first.

DROP TABLE IF EXISTS user_guilds;
DROP TABLE IF EXISTS discord_tokens;
DROP TABLE IF EXISTS oauth_states;
