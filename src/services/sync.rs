use sqlx::PgPool;

use crate::error::AppError;
use crate::models::condition::Condition;
use crate::services::condition_eval::evaluate_conditions;
use crate::services::rolelogic::RoleLogicClient;

#[derive(Debug, Clone)]
pub enum SyncEvent {
    PlayerUpdated { discord_id: String },
    ConfigChanged { guild_id: String, role_id: String },
    AccountLinked { discord_id: String },
    AccountUnlinked { discord_id: String },
}

/// Sync roles for a single player across all guilds.
pub async fn sync_for_player(
    discord_id: &str,
    pool: &PgPool,
    rl_client: &RoleLogicClient,
) -> Result<(), AppError> {
    // Get player's cached data
    let cache = sqlx::query_as::<_, (serde_json::Value, Option<String>)>(
        "SELECT pc.player_info, pc.region FROM player_cache pc \
         JOIN linked_accounts la ON la.uid = pc.uid \
         WHERE la.discord_id = $1",
    )
    .bind(discord_id)
    .fetch_optional(pool)
    .await?;

    let Some((player_info, region)) = cache else {
        return Ok(());
    };

    // Get all role links
    let role_links = sqlx::query_as::<_, (String, String, String, sqlx::types::Json<Vec<Condition>>)>(
        "SELECT guild_id, role_id, api_token, conditions FROM role_links",
    )
    .fetch_all(pool)
    .await?;

    for (guild_id, role_id, api_token, conditions) in &role_links {
        let qualifies = evaluate_conditions(conditions, &player_info, region.as_deref());

        let currently_assigned = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM role_assignments WHERE guild_id = $1 AND role_id = $2 AND discord_id = $3)",
        )
        .bind(guild_id)
        .bind(role_id)
        .bind(discord_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        match (qualifies, currently_assigned) {
            (true, false) => {
                match rl_client.add_user(guild_id, role_id, discord_id, api_token).await {
                    Err(AppError::UserLimitReached { limit }) => {
                        tracing::warn!(
                            guild_id, role_id, discord_id, limit,
                            "Cannot add user: role link user limit reached"
                        );
                        continue;
                    }
                    Err(e) => {
                        tracing::error!(
                            guild_id, role_id, discord_id,
                            "Failed to add user to role: {e}"
                        );
                        continue;
                    }
                    Ok(_) => {}
                }
                sqlx::query(
                    "INSERT INTO role_assignments (guild_id, role_id, discord_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(guild_id)
                .bind(role_id)
                .bind(discord_id)
                .execute(pool)
                .await?;
            }
            (false, true) => {
                if let Err(e) = rl_client.remove_user(guild_id, role_id, discord_id, api_token).await {
                    tracing::error!(
                        guild_id, role_id, discord_id,
                        "Failed to remove user from role: {e}"
                    );
                    continue;
                }
                sqlx::query(
                    "DELETE FROM role_assignments WHERE guild_id = $1 AND role_id = $2 AND discord_id = $3",
                )
                .bind(guild_id)
                .bind(role_id)
                .bind(discord_id)
                .execute(pool)
                .await?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Re-evaluate all users for a specific role link (after config change).
/// Uses atomic PUT to replace entire user list, respecting the role link's user limit.
pub async fn sync_for_role_link(
    guild_id: &str,
    role_id: &str,
    pool: &PgPool,
    rl_client: &RoleLogicClient,
) -> Result<(), AppError> {
    let link = sqlx::query_as::<_, (String, sqlx::types::Json<Vec<Condition>>)>(
        "SELECT api_token, conditions FROM role_links WHERE guild_id = $1 AND role_id = $2",
    )
    .bind(guild_id)
    .bind(role_id)
    .fetch_optional(pool)
    .await?;

    let Some((api_token, conditions)) = link else {
        return Ok(());
    };

    // Query the user limit from RoleLogic
    let (_user_count, user_limit) = rl_client
        .get_user_info(guild_id, role_id, &api_token)
        .await
        .unwrap_or((0, 100)); // Default to 100 (free plan) if query fails

    // Get all linked players with cached data, ordered by linked_at for FIFO priority
    let players = sqlx::query_as::<_, (String, serde_json::Value, Option<String>)>(
        "SELECT la.discord_id, pc.player_info, pc.region \
         FROM linked_accounts la \
         JOIN player_cache pc ON pc.uid = la.uid \
         ORDER BY la.linked_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let qualifying_ids: Vec<String> = players
        .into_iter()
        .filter(|(_, player_info, region)| {
            evaluate_conditions(&conditions, player_info, region.as_deref())
        })
        .map(|(discord_id, _, _)| discord_id)
        .collect();

    let total_qualifying = qualifying_ids.len();

    // Truncate to user limit (FIFO: earliest-linked users get priority)
    let synced_ids: Vec<String> = qualifying_ids.into_iter().take(user_limit).collect();

    if total_qualifying > user_limit {
        tracing::warn!(
            guild_id, role_id, total_qualifying, user_limit,
            "Role link user limit reached: {total_qualifying} users qualify but limit is {user_limit}, synced first {user_limit}"
        );
    }

    // Atomic replace
    rl_client
        .replace_users(guild_id, role_id, &synced_ids, &api_token)
        .await?;

    // Update local assignments to match what was actually sent
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM role_assignments WHERE guild_id = $1 AND role_id = $2")
        .bind(guild_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    for user_id in &synced_ids {
        sqlx::query(
            "INSERT INTO role_assignments (guild_id, role_id, discord_id) VALUES ($1, $2, $3)",
        )
        .bind(guild_id)
        .bind(role_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Remove a user from all role assignments (after account unlink).
pub async fn remove_all_assignments(
    discord_id: &str,
    pool: &PgPool,
    rl_client: &RoleLogicClient,
) -> Result<(), AppError> {
    let assignments = sqlx::query_as::<_, (String, String, String)>(
        "SELECT ra.guild_id, ra.role_id, rl.api_token \
         FROM role_assignments ra \
         JOIN role_links rl ON rl.guild_id = ra.guild_id AND rl.role_id = ra.role_id \
         WHERE ra.discord_id = $1",
    )
    .bind(discord_id)
    .fetch_all(pool)
    .await?;

    for (guild_id, role_id, api_token) in &assignments {
        if let Err(e) = rl_client
            .remove_user(guild_id, role_id, discord_id, api_token)
            .await
        {
            tracing::error!(
                guild_id, role_id, discord_id,
                "Failed to remove user during unlink: {e}"
            );
        }
    }

    sqlx::query("DELETE FROM role_assignments WHERE discord_id = $1")
        .bind(discord_id)
        .execute(pool)
        .await?;

    Ok(())
}
