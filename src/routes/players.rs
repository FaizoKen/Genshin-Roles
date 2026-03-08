use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::AppState;

#[derive(Deserialize)]
pub struct PlayersQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    sort: Option<String>,
    order: Option<String>,
    search: Option<String>,
}

fn sort_column(key: &str) -> Option<&'static str> {
    match key {
        "level" => Some("pc.level"),
        "world_level" => Some("pc.world_level"),
        "achievements" => Some("pc.achievements"),
        "abyss_progress" => Some("pc.abyss_progress"),
        "fetter_count" => Some("pc.fetter_count"),
        "region" => Some("pc.region"),
        "fetched_at" => Some("pc.fetched_at"),
        "uid" => Some("pc.uid"),
        "nickname" => Some("pc.player_info->>'nickname'"),
        "discord_name" => Some("la.discord_name"),
        _ => None,
    }
}

pub fn render_players_page(base_url: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Genshin Roles - Player List</title>
    <link rel="icon" href="{base_url}/favicon.ico" type="image/x-icon">
    <meta name="description" content="View verified Genshin Impact players in this Discord Server.">
    <meta name="theme-color" content="#e8b44a">
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 1060px; margin: 0 auto; padding: 32px 20px; background: #0e1525; color: #c8ccd4; min-height: 100vh; }}
        .header {{ margin-bottom: 24px; }}
        .header-top {{ display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }}
        .header-top h1 {{ color: #e8b44a; font-size: 24px; }}
        .powered {{ font-size: 11px; color: #64748b; background: #1e293b; padding: 2px 8px; border-radius: 4px; }}
        .powered a {{ color: #74b9ff; text-decoration: none; }}
        .guild-name {{ color: #e2e8f0; font-size: 18px; font-weight: 600; }}
        .guild-label {{ color: #64748b; font-size: 13px; margin-top: 2px; }}

        .card {{ background: #161d2e; padding: 22px; border-radius: 10px; border: 1px solid #1e2a3d; }}
        .msg {{ padding: 10px 14px; border-radius: 6px; margin: 12px 0; font-size: 13px; line-height: 1.5; }}
        .msg-error {{ background: #1c0a0a; color: #fca5a5; border: 1px solid #7f1d1d; }}
        .hidden {{ display: none !important; }}

        .toolbar {{ display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px; margin-bottom: 16px; }}
        .search-wrap {{ position: relative; flex: 1; max-width: 340px; }}
        .search-wrap svg {{ position: absolute; left: 10px; top: 50%; transform: translateY(-50%); color: #475569; pointer-events: none; }}
        .search-wrap input {{ width: 100%; padding: 8px 12px 8px 34px; font-size: 13px; border-radius: 6px; border: 1px solid #2a3548; background: #0e1525; color: #e0e0e0; font-family: inherit; transition: border-color .15s; }}
        .search-wrap input:focus {{ outline: none; border-color: #3b82f6; }}
        .search-hint {{ color: #475569; font-size: 11px; margin-top: 4px; }}
        .badge {{ display: inline-flex; align-items: center; gap: 5px; padding: 4px 12px; border-radius: 20px; font-size: 12px; font-weight: 500; background: #1e293b; color: #94a3b8; border: 1px solid #334155; white-space: nowrap; }}

        .table-wrap {{ overflow-x: auto; }}
        table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
        th, td {{ padding: 9px 12px; text-align: left; white-space: nowrap; }}
        th {{ color: #64748b; font-weight: 600; font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px; border-bottom: 2px solid #1e2a3d; cursor: pointer; user-select: none; transition: color .15s; }}
        th:hover {{ color: #94a3b8; }}
        th.sorted-asc::after {{ content: ' \25B2'; font-size: 9px; }}
        th.sorted-desc::after {{ content: ' \25BC'; font-size: 9px; }}
        td {{ border-bottom: 1px solid #111827; }}
        tr:hover td {{ background: #1a2236; }}
        .col-nickname {{ color: #e2e8f0; font-weight: 500; }}
        .col-uid {{ font-family: 'Courier New', monospace; }}
        .col-uid a {{ color: #74b9ff; text-decoration: none; }}
        .col-uid a:hover {{ text-decoration: underline; }}
        .col-discord {{ color: #7c85f5; font-size: 12px; }}
        .col-region {{ color: #94a3b8; }}
        .col-num {{ color: #e8b44a; text-align: right; }}
        th.col-num {{ text-align: right; }}
        .col-date {{ color: #64748b; font-size: 12px; }}
        .abyss {{ color: #c084fc; }}

        .empty-state {{ text-align: center; padding: 40px 20px; color: #475569; }}
        .empty-state p {{ font-size: 14px; margin-bottom: 4px; }}
        .empty-state .hint {{ font-size: 12px; }}

        .pagination {{ display: flex; align-items: center; justify-content: center; gap: 8px; margin-top: 16px; font-size: 13px; }}
        .pagination button {{ padding: 6px 14px; border-radius: 6px; border: 1px solid #2a3548; background: #0e1525; color: #c8ccd4; cursor: pointer; font-family: inherit; font-size: 13px; transition: all .15s; }}
        .pagination button:hover:not(:disabled) {{ background: #1e293b; border-color: #3b82f6; }}
        .pagination button:disabled {{ opacity: 0.3; cursor: not-allowed; }}
        .pagination .page-info {{ color: #64748b; }}
    </style>
</head>
<body>
    <div class="header">
        <div class="header-top">
            <h1>Genshin Roles</h1>
            <span class="powered">Powered by <a href="https://rolelogic.faizo.net" target="_blank" rel="noopener">RoleLogic</a></span>
        </div>
        <p class="guild-name" id="guild-name">Verified Players</p>
        <p class="guild-label" id="guild-label">Loading guild info...</p>
    </div>

    <div id="loading" class="card"><p style="color:#64748b;">Loading player data...</p></div>
    <div id="error-msg" class="hidden"></div>

    <div id="content" class="hidden">
        <div class="card">
            <div class="toolbar">
                <div>
                    <div class="search-wrap">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
                        <input type="text" id="search" placeholder="Search players..." />
                    </div>
                    <p class="search-hint">Search by nickname, UID, Discord name, or Discord ID</p>
                </div>
                <span class="badge" id="player-count"></span>
            </div>
            <div class="table-wrap">
                <table>
                    <thead>
                        <tr>
                            <th data-key="nickname">Nickname</th>
                            <th data-key="discord_name">Discord</th>
                            <th data-key="uid">UID</th>
                            <th data-key="level" class="col-num">AR</th>
                            <th data-key="world_level" class="col-num">WL</th>
                            <th data-key="achievements" class="col-num">Achievements</th>
                            <th data-key="abyss_progress" class="col-num">Abyss</th>
                            <th data-key="fetter_count" class="col-num">Friendship</th>
                            <th data-key="region">Region</th>
                            <th data-key="fetched_at">Last Updated</th>
                        </tr>
                    </thead>
                    <tbody id="tbody"></tbody>
                </table>
            </div>
            <div id="empty-state" class="empty-state hidden">
                <p>No players found</p>
                <p class="hint" id="empty-hint">Try a different search term</p>
            </div>
            <div class="pagination" id="pagination">
                <button id="btn-prev" onclick="goPage(state.page-1)">Prev</button>
                <span class="page-info" id="page-info"></span>
                <button id="btn-next" onclick="goPage(state.page+1)">Next</button>
            </div>
        </div>
    </div>

    <script>
    const guildId = window.location.pathname.split('/').filter(Boolean)[1] || '';
    const PER_PAGE = 20;
    const NUM_COLS = ['level','world_level','achievements','abyss_progress','fetter_count'];

    const state = {{ page: 1, sort: 'level', order: 'desc', search: '', total: 0 }};
    let debounceTimer = null;

    function formatAbyss(val) {{
        if (!val) return '-';
        const floor = Math.floor(val / 10);
        const chamber = val % 10;
        return floor + '-' + chamber;
    }}

    function timeAgo(iso) {{
        if (!iso) return '-';
        const diff = Date.now() - new Date(iso).getTime();
        const mins = Math.floor(diff / 60000);
        if (mins < 1) return 'just now';
        if (mins < 60) return mins + 'm ago';
        const hrs = Math.floor(mins / 60);
        if (hrs < 24) return hrs + 'h ago';
        const days = Math.floor(hrs / 24);
        return days + 'd ago';
    }}

    function esc(s) {{
        const d = document.createElement('div');
        d.textContent = s;
        return d.innerHTML;
    }}

    function render(players) {{
        const tbody = document.getElementById('tbody');
        const emptyEl = document.getElementById('empty-state');
        tbody.innerHTML = '';
        if (players.length === 0) {{
            emptyEl.classList.remove('hidden');
            document.getElementById('empty-hint').textContent = state.search
                ? 'No results for "' + state.search + '"'
                : 'No verified players in this guild yet';
        }} else {{
            emptyEl.classList.add('hidden');
        }}
        players.forEach(p => {{
            const tr = document.createElement('tr');
            const discord = p.discord_name || p.discord_id || '-';
            tr.innerHTML =
                '<td class="col-nickname">' + esc(p.nickname || '-') + '</td>' +
                '<td class="col-discord">' + esc(discord) + '</td>' +
                '<td class="col-uid"><a href="https://enka.network/u/' + esc(p.uid) + '" target="_blank" rel="noopener">' + esc(p.uid) + '</a></td>' +
                '<td class="col-num">' + p.level + '</td>' +
                '<td class="col-num">' + p.world_level + '</td>' +
                '<td class="col-num">' + p.achievements + '</td>' +
                '<td class="col-num abyss">' + formatAbyss(p.abyss_progress) + '</td>' +
                '<td class="col-num">' + p.fetter_count + '</td>' +
                '<td class="col-region">' + esc(p.region || '-') + '</td>' +
                '<td class="col-date">' + timeAgo(p.fetched_at) + '</td>';
            tbody.appendChild(tr);
        }});
    }}

    function updatePagination() {{
        const totalPages = Math.max(1, Math.ceil(state.total / PER_PAGE));
        document.getElementById('player-count').textContent = state.total + ' player' + (state.total !== 1 ? 's' : '');
        document.getElementById('page-info').textContent = 'Page ' + state.page + ' of ' + totalPages;
        document.getElementById('btn-prev').disabled = state.page <= 1;
        document.getElementById('btn-next').disabled = state.page >= totalPages;
        document.getElementById('pagination').classList.toggle('hidden', state.total <= PER_PAGE);
    }}

    function updateSortUI() {{
        document.querySelectorAll('th[data-key]').forEach(h => {{
            h.classList.remove('sorted-asc', 'sorted-desc');
            if (h.dataset.key === state.sort) h.classList.add('sorted-' + state.order);
        }});
    }}

    async function fetchData() {{
        const params = new URLSearchParams({{
            page: state.page, per_page: PER_PAGE,
            sort: state.sort, order: state.order
        }});
        if (state.search) params.set('search', state.search);
        const res = await fetch('/players/' + encodeURIComponent(guildId) + '/data?' + params);
        if (!res.ok) {{
            const data = await res.json().catch(() => ({{}}));
            throw new Error(data.error || 'Failed to load player data');
        }}
        return res.json();
    }}

    async function load() {{
        try {{
            const data = await fetchData();
            state.total = data.total;
            if (data.guild_name) {{
                document.getElementById('guild-name').textContent = data.guild_name;
                document.getElementById('guild-label').textContent = 'Verified Genshin Impact players';
                document.title = data.guild_name + ' - Genshin Roles';
            }} else {{
                document.getElementById('guild-name').textContent = 'Verified Players';
                document.getElementById('guild-label').textContent = 'Genshin Impact player list';
            }}
            render(data.players);
            updatePagination();
            updateSortUI();
            document.getElementById('loading').classList.add('hidden');
            document.getElementById('content').classList.remove('hidden');
            document.getElementById('error-msg').classList.add('hidden');
        }} catch (e) {{
            document.getElementById('loading').classList.add('hidden');
            const el = document.getElementById('error-msg');
            el.className = 'msg msg-error';
            el.textContent = e.message;
            el.classList.remove('hidden');
        }}
    }}

    function goPage(p) {{
        const totalPages = Math.max(1, Math.ceil(state.total / PER_PAGE));
        state.page = Math.max(1, Math.min(p, totalPages));
        load();
    }}

    document.querySelectorAll('th[data-key]').forEach(th => {{
        th.addEventListener('click', () => {{
            const key = th.dataset.key;
            if (state.sort === key) {{
                state.order = state.order === 'asc' ? 'desc' : 'asc';
            }} else {{
                state.sort = key;
                state.order = NUM_COLS.includes(key) ? 'desc' : 'asc';
            }}
            state.page = 1;
            load();
        }});
    }});

    document.getElementById('search').addEventListener('input', e => {{
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {{
            state.search = e.target.value.trim();
            state.page = 1;
            load();
        }}, 300);
    }});

    load();
    </script>
</body>
</html>"##
    )
}

pub async fn players_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        state.players_html.clone(),
    )
}

pub async fn players_data(
    State(state): State<Arc<AppState>>,
    Path(guild_id): Path<String>,
    Query(query): Query<PlayersQuery>,
) -> Result<Json<Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let order_col = query
        .sort
        .as_deref()
        .and_then(sort_column)
        .unwrap_or("pc.level");
    let order_dir = match query.order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let search = query.search.as_deref().unwrap_or("").trim();
    let has_search = !search.is_empty();
    let search_pattern = format!("%{search}%");

    // Build query with optional search filter
    let sql = format!(
        "SELECT pc.uid, \
                pc.player_info, \
                pc.level, \
                pc.world_level, \
                pc.achievements, \
                pc.abyss_progress, \
                pc.fetter_count, \
                pc.region, \
                pc.fetched_at, \
                la.discord_id, \
                la.discord_name, \
                COUNT(*) OVER() AS total_count, \
                (SELECT ug2.guild_name FROM user_guilds ug2 WHERE ug2.guild_id = $1 AND ug2.guild_name IS NOT NULL LIMIT 1) AS guild_name \
         FROM user_guilds ug \
         JOIN linked_accounts la ON la.discord_id = ug.discord_id \
         JOIN player_cache pc ON pc.uid = la.uid \
         WHERE ug.guild_id = $1 {search_clause} \
         ORDER BY {order_col} {order_dir} NULLS LAST \
         LIMIT $2 OFFSET $3",
        search_clause = if has_search {
            "AND (pc.player_info->>'nickname' ILIKE $4 \
             OR pc.uid ILIKE $4 \
             OR la.discord_name ILIKE $4 \
             OR la.discord_id ILIKE $4)"
        } else {
            ""
        },
        order_col = order_col,
        order_dir = order_dir,
    );

    use sqlx::Row;
    let rows = if has_search {
        sqlx::query(&sql)
            .bind(&guild_id)
            .bind(per_page)
            .bind(offset)
            .bind(&search_pattern)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query(&sql)
            .bind(&guild_id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
    };

    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    let guild_name: Option<String> = rows.first().and_then(|r| r.get("guild_name"));

    let players: Vec<Value> = rows
        .iter()
        .map(|r| {
            let player_info: Value = r.get("player_info");
            let nickname = player_info.get("nickname").and_then(|v| v.as_str());
            let fetched_at: chrono::DateTime<chrono::Utc> = r.get("fetched_at");
            json!({
                "uid": r.get::<String, _>("uid"),
                "nickname": nickname,
                "level": r.get::<i32, _>("level"),
                "world_level": r.get::<i32, _>("world_level"),
                "achievements": r.get::<i32, _>("achievements"),
                "abyss_progress": r.get::<i32, _>("abyss_progress"),
                "fetter_count": r.get::<i32, _>("fetter_count"),
                "region": r.get::<Option<String>, _>("region"),
                "fetched_at": fetched_at,
                "discord_id": r.get::<String, _>("discord_id"),
                "discord_name": r.get::<Option<String>, _>("discord_name"),
            })
        })
        .collect();

    Ok(Json(json!({
        "players": players,
        "total": total,
        "page": page,
        "per_page": per_page,
        "guild_name": guild_name,
    })))
}
