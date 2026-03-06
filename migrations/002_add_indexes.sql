CREATE INDEX IF NOT EXISTS idx_role_links_api_token ON role_links (api_token);
CREATE INDEX IF NOT EXISTS idx_role_assignments_discord_id ON role_assignments (discord_id);
