use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .is_ok();

    Json(json!({
        "status": if db_ok { "healthy" } else { "degraded" },
        "database": db_ok,
    }))
}
