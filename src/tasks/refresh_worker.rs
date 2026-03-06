use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::error::EnkaError;
use crate::services::sync::SyncEvent;
use crate::AppState;

/// Target: max 60 Enka requests per hour (well within limits).
/// Interval scales with player count so load stays constant.
const MAX_REQUESTS_PER_HOUR: i64 = 60;
const MIN_REFRESH_SECS: i64 = 1800; // 30 min floor
const MAX_REFRESH_SECS: i64 = 86400; // 24 hour cap
const INTERVAL_CACHE_SECS: u64 = 300; // recompute every 5 minutes

/// Caches the refresh interval to avoid running COUNT(*) on every fetch cycle.
struct CachedInterval {
    value: AtomicI64,
    last_computed: Mutex<Instant>,
}

impl CachedInterval {
    fn new() -> Self {
        Self {
            value: AtomicI64::new(MIN_REFRESH_SECS),
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
                ((player_count * 3600) / MAX_REQUESTS_PER_HOUR).clamp(MIN_REFRESH_SECS, MAX_REFRESH_SECS)
            };

            self.value.store(interval, Ordering::Relaxed);
            *last = Instant::now();
        }
        self.value.load(Ordering::Relaxed)
    }
}

pub async fn run(state: Arc<AppState>) {
    tracing::info!("Refresh worker started");

    let cached_interval = CachedInterval::new();

    loop {
        // Wait for rate limiter
        state.enka_client.wait_for_permit().await;

        // Get next UID due for refresh
        let next = sqlx::query_as::<_, (String, String)>(
            "SELECT pc.uid, la.discord_id FROM player_cache pc \
             JOIN linked_accounts la ON la.uid = pc.uid \
             WHERE pc.next_fetch_at <= now() \
             ORDER BY pc.fetch_failures ASC, pc.next_fetch_at ASC \
             LIMIT 1",
        )
        .fetch_optional(&state.pool)
        .await;

        let (uid, discord_id) = match next {
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

        tracing::debug!(uid, "Refreshing player data");

        match state.enka_client.fetch_player_info(&uid).await {
            Ok(response) => {
                // Scale refresh interval based on total player count (cached)
                let interval = cached_interval.get(&state.pool).await;
                let ttl = (response.ttl as i64).max(interval);
                let next_fetch = chrono::Utc::now() + chrono::Duration::seconds(ttl);

                if let Err(e) = sqlx::query(
                    "UPDATE player_cache SET \
                     player_info = $1, region = $2, enka_ttl = $3, \
                     fetched_at = now(), next_fetch_at = $4, fetch_failures = 0 \
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
                    .sync_tx
                    .send(SyncEvent::PlayerUpdated {
                        discord_id: discord_id.clone(),
                    })
                    .await;

                tracing::debug!(uid, ttl, "Player data refreshed");
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
