use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LinkedAccount {
    pub id: i64,
    pub discord_id: String,
    pub uid: String,
    pub linked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlayerCache {
    pub uid: String,
    pub player_info: serde_json::Value,
    pub region: Option<String>,
    pub enka_ttl: i32,
    pub fetched_at: DateTime<Utc>,
    pub next_fetch_at: DateTime<Utc>,
    pub fetch_failures: i32,
}
