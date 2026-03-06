use std::sync::Arc;

use crate::error::EnkaError;
use crate::services::sync::SyncEvent;
use crate::AppState;

pub async fn run(state: Arc<AppState>) {
    tracing::info!("Refresh worker started");

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
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
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
                let ttl = response.ttl.max(60);
                let next_fetch = chrono::Utc::now() + chrono::Duration::seconds(ttl as i64);

                if let Err(e) = sqlx::query(
                    "UPDATE player_cache SET \
                     player_info = $1, region = $2, enka_ttl = $3, \
                     fetched_at = now(), next_fetch_at = $4, fetch_failures = 0 \
                     WHERE uid = $5",
                )
                .bind(&response.player_info)
                .bind(&response.region)
                .bind(ttl)
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
