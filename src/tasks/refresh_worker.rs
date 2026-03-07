use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::error::EnkaError;
use crate::services::sync::PlayerSyncEvent;
use crate::AppState;

/// Configurable via ENKA_MAX_REQUESTS_PER_HOUR env var.
/// Default 360 (6/min) -- respectful sustained rate per Enka guidance.
fn max_requests_per_hour() -> i64 {
    std::env::var("ENKA_MAX_REQUESTS_PER_HOUR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(360)
}

const MIN_REFRESH_SECS: i64 = 1800; // 30 min floor
const MAX_REFRESH_SECS: i64 = 86400; // 24 hour cap
const INTERVAL_CACHE_SECS: u64 = 300; // recompute every 5 minutes

/// Inactive players (no role_assignments) are refreshed this many times slower.
const INACTIVE_MULTIPLIER: i64 = 6;

/// Caches the refresh interval to avoid running COUNT(*) on every fetch cycle.
struct CachedInterval {
    value: AtomicI64,
    max_req_per_hour: i64,
    last_computed: Mutex<Instant>,
}

impl CachedInterval {
    fn new(max_req_per_hour: i64) -> Self {
        Self {
            value: AtomicI64::new(MIN_REFRESH_SECS),
            max_req_per_hour,
            // Start in the past so the first call triggers a recompute
            last_computed: Mutex::new(Instant::now() - std::time::Duration::from_secs(INTERVAL_CACHE_SECS + 1)),
        }
    }

    async fn get(&self, pool: &sqlx::PgPool) -> i64 {
        let mut last = self.last_computed.lock().await;
        if last.elapsed() >= std::time::Duration::from_secs(INTERVAL_CACHE_SECS) {
            let player_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM linked_accounts")
                .fetch_one(pool)
                .await
                .unwrap_or(0);

            let interval = if player_count == 0 {
                MIN_REFRESH_SECS
            } else {
                ((player_count * 3600) / self.max_req_per_hour).clamp(MIN_REFRESH_SECS, MAX_REFRESH_SECS)
            };

            self.value.store(interval, Ordering::Relaxed);
            *last = Instant::now();
        }
        self.value.load(Ordering::Relaxed)
    }
}

pub async fn run(state: Arc<AppState>) {
    let max_req = max_requests_per_hour();
    tracing::info!(max_req, "Refresh worker started");

    let cached_interval = CachedInterval::new(max_req);

    loop {
        // Wait for rate limiter
        state.enka_client.wait_for_permit().await;

        // Get next UID due for refresh, prioritizing active players (those with role_assignments)
        let next = sqlx::query_as::<_, (String, String, bool)>(
            "SELECT pc.uid, la.discord_id, \
             EXISTS(SELECT 1 FROM role_assignments ra WHERE ra.discord_id = la.discord_id) as is_active \
             FROM player_cache pc \
             JOIN linked_accounts la ON la.uid = pc.uid \
             WHERE pc.next_fetch_at <= now() \
             ORDER BY \
               (CASE WHEN EXISTS(SELECT 1 FROM role_assignments ra WHERE ra.discord_id = la.discord_id) THEN 0 ELSE 1 END) ASC, \
               pc.fetch_failures ASC, \
               pc.next_fetch_at ASC \
             LIMIT 1",
        )
        .fetch_optional(&state.pool)
        .await;

        let (uid, discord_id, is_active) = match next {
            Ok(Some(row)) => row,
            Ok(None) => {
                // Nothing to refresh, sleep
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                continue;
            }
            Err(e) => {
                tracing::error!("Refresh worker DB error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        tracing::debug!(uid, is_active, "Refreshing player data");

        match state.enka_client.fetch_player_info(&uid).await {
            Ok(response) => {
                // Scale refresh interval: active players get base interval,
                // inactive players get INACTIVE_MULTIPLIER times longer
                let base_interval = cached_interval.get(&state.pool).await;
                let multiplier = if is_active { 1 } else { INACTIVE_MULTIPLIER };
                let interval = base_interval * multiplier;
                let ttl = (response.ttl as i64).max(interval);
                let next_fetch = chrono::Utc::now() + chrono::Duration::seconds(ttl);

                if let Err(e) = sqlx::query(
                    "UPDATE player_cache SET \
                     player_info = $1, region = $2, enka_ttl = $3, \
                     fetched_at = now(), next_fetch_at = $4, fetch_failures = 0, \
                     level = COALESCE(($1->>'level')::int, 0), \
                     world_level = COALESCE(($1->>'worldLevel')::int, 0), \
                     achievements = COALESCE(($1->>'finishAchievementNum')::int, 0), \
                     tower_floor = COALESCE(($1->>'towerFloorIndex')::int, 0), \
                     tower_level = COALESCE(($1->>'towerLevelIndex')::int, 0), \
                     fetter_count = COALESCE(($1->>'fetterCount')::int, 0) \
                     WHERE uid = $5",
                )
                .bind(&response.player_info)
                .bind(&response.region)
                .bind(ttl as i32)
                .bind(next_fetch)
                .bind(&uid)
                .execute(&state.pool)
                .await
                {
                    tracing::error!(uid, "Failed to update player cache: {e}");
                    continue;
                }

                // Trigger sync for this player
                let _ = state
                    .player_sync_tx
                    .send(PlayerSyncEvent::PlayerUpdated {
                        discord_id: discord_id.clone(),
                    })
                    .await;

                tracing::debug!(uid, ttl, is_active, "Player data refreshed");
            }
            Err(EnkaError::RateLimited) => {
                tracing::warn!("Enka rate limited, backing off 5s");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(EnkaError::Maintenance) => {
                tracing::warn!("Enka maintenance, backing off 10min");
                // Push back all pending refreshes by 10 minutes
                let backoff = chrono::Utc::now() + chrono::Duration::minutes(10);
                let _ = sqlx::query(
                    "UPDATE player_cache SET next_fetch_at = $1 WHERE next_fetch_at <= now()",
                )
                .bind(backoff)
                .execute(&state.pool)
                .await;
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            }
            Err(e) => {
                // Exponential backoff for this UID
                let failures = sqlx::query_scalar::<_, i32>(
                    "UPDATE player_cache SET fetch_failures = fetch_failures + 1, \
                     next_fetch_at = now() + LEAST(INTERVAL '60 seconds' * POWER(2, fetch_failures), INTERVAL '1 hour') \
                     WHERE uid = $1 \
                     RETURNING fetch_failures",
                )
                .bind(&uid)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten()
                .unwrap_or(0);

                tracing::warn!(uid, failures, "Enka fetch failed: {e}");
            }
        }
    }
}
