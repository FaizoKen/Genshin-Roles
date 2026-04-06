-- Player list view permission: who can view the verified-player page for a guild.
-- 'members'  = any logged-in Discord user who is a member of the guild
-- 'managers' = only users with the Discord MANAGE_GUILD permission (or owner)
ALTER TABLE role_links
    ADD COLUMN IF NOT EXISTS view_permission TEXT NOT NULL DEFAULT 'members';

-- Track whether a user has MANAGE_GUILD permission in a given guild.
-- Populated by the Auth Gateway's guild refresh flow.
ALTER TABLE user_guilds
    ADD COLUMN IF NOT EXISTS manage_guild BOOLEAN NOT NULL DEFAULT FALSE;
