use std::collections::HashSet;

use futures_util::stream::{self, StreamExt};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::condition::{Condition, ConditionField};
use crate::services::condition_eval::evaluate_conditions;
use crate::services::rolelogic::RoleLogicClient;

/// Events sent to the player sync worker (lightweight, per-user).
#[derive(Debug, Clone)]
pub enum PlayerSyncEvent {
    PlayerUpdated { discord_id: String },
    AccountLinked { discord_id: String },
    AccountUnlinked { discord_id: String },
}

/// Events sent to the config sync worker (heavy, per-role-link).
#[derive(Debug, Clone)]
pub struct ConfigSyncEvent {
    pub guild_id: String,
    pub role_id: String,
}

/// Sync roles for a single player across all guilds.
/// Evaluates conditions locally (microseconds per role link), then executes
/// RoleLogic API calls concurrently for any changes needed.
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

    // Get role links only for guilds this user is a member of
    let role_links = sqlx::query_as::<_, (String, String, String, sqlx::types::Json<Vec<Condition>>)>(
        "SELECT rl.guild_id, rl.role_id, rl.api_token, rl.conditions \
         FROM role_links rl \
         JOIN user_guilds ug ON ug.guild_id = rl.guild_id \
         WHERE ug.discord_id = $1",
    )
    .bind(discord_id)
    .fetch_all(pool)
    .await?;

    // Batch: fetch all existing assignments for this user in ONE query
    let existing: HashSet<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT guild_id, role_id FROM role_assignments WHERE discord_id = $1",
    )
    .bind(discord_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

    // Phase 1: evaluate all conditions locally (no I/O, microseconds each)
    enum Action {
        Add { guild_id: String, role_id: String, api_token: String },
        Remove { guild_id: String, role_id: String, api_token: String },
    }

    let mut actions: Vec<Action> = Vec::new();
    for (guild_id, role_id, api_token, conditions) in &role_links {
        let qualifies = evaluate_conditions(conditions, &player_info, region.as_deref());
        let currently_assigned = existing.contains(&(guild_id.clone(), role_id.clone()));
        match (qualifies, currently_assigned) {
            (true, false) => actions.push(Action::Add {
                guild_id: guild_id.clone(),
                role_id: role_id.clone(),
                api_token: api_token.clone(),
            }),
            (false, true) => actions.push(Action::Remove {
                guild_id: guild_id.clone(),
                role_id: role_id.clone(),
                api_token: api_token.clone(),
            }),
            _ => {}
        }
    }

    if actions.is_empty() {
        return Ok(());
    }

    // Phase 2: execute API calls concurrently (max 10 parallel)
    let discord_id_owned = discord_id.to_string();
    stream::iter(actions)
        .for_each_concurrent(10, |action| {
            let pool = pool.clone();
            let rl_client = rl_client.clone();
            let discord_id = discord_id_owned.clone();
            async move {
                match action {
                    Action::Add { guild_id, role_id, api_token } => {
                        match rl_client.add_user(&guild_id, &role_id, &discord_id, &api_token).await {
                            Err(AppError::UserLimitReached { limit }) => {
                                tracing::warn!(
                                    guild_id, role_id, discord_id, limit,
                                    "Cannot add user: role link user limit reached"
                                );
                                return;
                            }
                            Err(e) => {
                                tracing::error!(
                                    guild_id, role_id, discord_id,
                                    "Failed to add user to role: {e}"
                                );
                                return;
                            }
                            Ok(_) => {}
                        }
                        if let Err(e) = sqlx::query(
                            "INSERT INTO role_assignments (guild_id, role_id, discord_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                        )
                        .bind(&guild_id)
                        .bind(&role_id)
                        .bind(&discord_id)
                        .execute(&pool)
                        .await {
                            tracing::error!(guild_id, role_id, discord_id, "Failed to insert assignment: {e}");
                        }
                    }
                    Action::Remove { guild_id, role_id, api_token } => {
                        if let Err(e) = rl_client.remove_user(&guild_id, &role_id, &discord_id, &api_token).await {
                            tracing::error!(
                                guild_id, role_id, discord_id,
                                "Failed to remove user from role: {e}"
                            );
                            return;
                        }
                        if let Err(e) = sqlx::query(
                            "DELETE FROM role_assignments WHERE guild_id = $1 AND role_id = $2 AND discord_id = $3",
                        )
                        .bind(&guild_id)
                        .bind(&role_id)
                        .bind(&discord_id)
                        .execute(&pool)
                        .await {
                            tracing::error!(guild_id, role_id, discord_id, "Failed to delete assignment: {e}");
                        }
                    }
                }
            }
        })
        .await;

    Ok(())
}

/// Build a SQL WHERE clause from conditions for SQL-side filtering.
/// Returns (where_clause_string, bind_values) where bind values are positional ($N).
/// Numeric/region conditions use extracted columns; HasAvatar/HasNameCard use JSONB operators.
fn build_condition_where(conditions: &[Condition]) -> (String, Vec<ConditionBind>) {
    if conditions.is_empty() {
        return ("TRUE".to_string(), vec![]);
    }

    let mut clauses: Vec<String> = Vec::new();
    let mut binds: Vec<ConditionBind> = Vec::new();

    for condition in conditions {
        match &condition.field {
            ConditionField::Region => {
                let val = condition.value.as_str().unwrap_or("").to_string();
                let idx = binds.len() + 1; // $1-based
                clauses.push(format!("LOWER(pc.region) = LOWER(${idx})"));
                binds.push(ConditionBind::Text(val));
            }
            ConditionField::HasAvatar => {
                let id = condition.value.as_i64().unwrap_or(0);
                let idx = binds.len() + 1;
                clauses.push(format!(
                    "pc.player_info->'showAvatarInfoList' @> concat('[{{\"avatarId\":', ${idx}::text, '}}]')::jsonb"
                ));
                binds.push(ConditionBind::Int(id));
            }
            ConditionField::HasNameCard => {
                let id = condition.value.as_i64().unwrap_or(0);
                let idx = binds.len() + 1;
                clauses.push(format!(
                    "pc.player_info->'showNameCardIdList' @> concat('[', ${idx}::text, ']')::jsonb"
                ));
                binds.push(ConditionBind::Int(id));
            }
            numeric_field => {
                let col = numeric_field.sql_column().unwrap(); // safe: Region handled above
                let op = condition.operator.sql_operator();
                let val = condition.value.as_i64().unwrap_or(0);
                let idx = binds.len() + 1;
                clauses.push(format!("{col} {op} ${idx}"));
                binds.push(ConditionBind::Int(val));
            }
        }
    }

    (clauses.join(" AND "), binds)
}

/// Bind value types for dynamic condition queries.
enum ConditionBind {
    Int(i64),
    Text(String),
}

/// Re-evaluate all users for a specific role link (after config change).
/// Uses SQL-side filtering on extracted columns to avoid streaming all JSONB blobs.
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

    // Build SQL WHERE clause from conditions -- pushes filtering to PostgreSQL
    let (where_clause, binds) = build_condition_where(&conditions);

    // Build the full query with LIMIT for user cap
    // JOIN user_guilds to only include users who are actually in this guild
    let guild_bind_idx = binds.len() + 1;
    let limit_bind_idx = binds.len() + 2;
    let query_str = format!(
        "SELECT la.discord_id \
         FROM linked_accounts la \
         JOIN player_cache pc ON pc.uid = la.uid \
         JOIN user_guilds ug ON ug.discord_id = la.discord_id AND ug.guild_id = ${guild_bind_idx} \
         WHERE {where_clause} \
         ORDER BY la.linked_at ASC \
         LIMIT ${limit_bind_idx}",
    );

    // We need to use a dynamic query builder since bind count varies
    let qualifying_ids = exec_condition_query(&query_str, &binds, guild_id, user_limit, pool).await?;

    // Check if more users qualify than the limit allows
    if !qualifying_ids.is_empty() && qualifying_ids.len() == user_limit {
        // Run a count query to see total qualifying (for logging only)
        let count_query = format!(
            "SELECT COUNT(*) FROM linked_accounts la \
             JOIN player_cache pc ON pc.uid = la.uid \
             JOIN user_guilds ug ON ug.discord_id = la.discord_id AND ug.guild_id = ${guild_bind_idx} \
             WHERE {where_clause}",
        );
        let total: i64 = exec_condition_count(&count_query, &binds, guild_id, pool)
            .await
            .unwrap_or(qualifying_ids.len() as i64);
        if total as usize > user_limit {
            tracing::warn!(
                guild_id, role_id, total, user_limit,
                "Role link user limit reached: {total} users qualify but limit is {user_limit}, synced first {user_limit}"
            );
        }
    }

    // Atomic replace
    rl_client
        .replace_users(guild_id, role_id, &qualifying_ids, &api_token)
        .await?;

    // Update local assignments to match what was actually sent
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM role_assignments WHERE guild_id = $1 AND role_id = $2")
        .bind(guild_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    if !qualifying_ids.is_empty() {
        sqlx::query(
            "INSERT INTO role_assignments (guild_id, role_id, discord_id) \
             SELECT $1, $2, UNNEST($3::text[])",
        )
        .bind(guild_id)
        .bind(role_id)
        .bind(&qualifying_ids)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Execute a dynamic condition query that returns discord_id strings.
/// Handles variable bind types and counts.
async fn exec_condition_query(
    query: &str,
    binds: &[ConditionBind],
    guild_id: &str,
    limit: usize,
    pool: &PgPool,
) -> Result<Vec<String>, AppError> {
    // sqlx doesn't support fully dynamic bind counts in a type-safe way,
    // so we build the query with raw SQL. Values are still parameterized
    // to prevent SQL injection.
    let mut q = sqlx::query_scalar::<_, String>(query);
    for bind in binds {
        q = match bind {
            ConditionBind::Int(v) => q.bind(*v),
            ConditionBind::Text(v) => q.bind(v),
        };
    }
    q = q.bind(guild_id);
    q = q.bind(limit as i64);

    Ok(q.fetch_all(pool).await?)
}

/// Execute a dynamic condition COUNT query.
async fn exec_condition_count(
    query: &str,
    binds: &[ConditionBind],
    guild_id: &str,
    pool: &PgPool,
) -> Result<i64, AppError> {
    let mut q = sqlx::query_scalar::<_, i64>(query);
    for bind in binds {
        q = match bind {
            ConditionBind::Int(v) => q.bind(*v),
            ConditionBind::Text(v) => q.bind(v),
        };
    }
    q = q.bind(guild_id);
    Ok(q.fetch_one(pool).await?)
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
