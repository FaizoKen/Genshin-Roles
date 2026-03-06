use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VerificationSession {
    pub id: i64,
    pub discord_id: String,
    pub uid: String,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthState {
    pub state: String,
    pub redirect_data: Option<serde_json::Value>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
