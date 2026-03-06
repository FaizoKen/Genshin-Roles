use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::condition::Condition;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoleLink {
    pub id: i64,
    pub guild_id: String,
    pub role_id: String,
    pub api_token: String,
    pub conditions: sqlx::types::Json<Vec<Condition>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
