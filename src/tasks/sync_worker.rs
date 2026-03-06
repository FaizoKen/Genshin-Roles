use std::sync::Arc;

use tokio::sync::mpsc;

use crate::services::sync::{self, SyncEvent};
use crate::AppState;

pub async fn run(mut rx: mpsc::Receiver<SyncEvent>, state: Arc<AppState>) {
    tracing::info!("Sync worker started");

    while let Some(event) = rx.recv().await {
        let result = match &event {
            SyncEvent::PlayerUpdated { discord_id } | SyncEvent::AccountLinked { discord_id } => {
                tracing::debug!(discord_id, event = ?event, "Syncing roles for player");
                sync::sync_for_player(discord_id, &state.pool, &state.rl_client).await
            }
            SyncEvent::ConfigChanged { guild_id, role_id } => {
                tracing::debug!(guild_id, role_id, "Syncing roles for config change");
                sync::sync_for_role_link(guild_id, role_id, &state.pool, &state.rl_client).await
            }
            SyncEvent::AccountUnlinked { discord_id } => {
                tracing::debug!(discord_id, "Removing all assignments for unlinked user");
                sync::remove_all_assignments(discord_id, &state.pool, &state.rl_client).await
            }
        };

        if let Err(e) = result {
            tracing::error!(event = ?event, "Sync failed: {e}");
        }
    }

    tracing::warn!("Sync worker channel closed");
}
