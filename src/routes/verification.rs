use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use rand::Rng;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::services::discord_oauth::{self, DiscordOAuth};
use crate::services::sync::PlayerSyncEvent;
use crate::AppState;

const SESSION_COOKIE: &str = "gr_session";

/// Returns (discord_id, display_name)
fn get_session(jar: &CookieJar, secret: &str) -> Result<(String, String), AppError> {
    let cookie = jar
        .get(SESSION_COOKIE)
        .ok_or(AppError::Unauthorized)?;

    discord_oauth::verify_session(cookie.value(), secret)
        .ok_or(AppError::Unauthorized)
}

fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    let code: String = (0..6).map(|_| chars[rng.gen_range(0..chars.len())]).collect();
    format!("GR-{code}")
}

fn validate_uid(uid: &str) -> Result<(), AppError> {
    if uid.len() < 9 || uid.len() > 10 {
        return Err(AppError::BadRequest("UID must be 9-10 digits".into()));
    }
    if !uid.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest("UID must contain only digits".into()));
    }
    Ok(())
}

pub fn render_verify_page(base_url: &str) -> String {
    let login_url = format!("{base_url}/verify/login");

    format!(
        r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Genshin Roles - Link Account</title>
    <link rel="icon" href="{base_url}/favicon.ico" type="image/x-icon">
    <meta name="description" content="Link your Discord account with your Genshin Impact UID to automatically receive server roles based on your in-game progress.">
    <meta property="og:type" content="website">
    <meta property="og:title" content="Genshin Roles - Link Account">
    <meta property="og:description" content="Link your Discord account with your Genshin Impact UID to automatically receive server roles based on your in-game progress.">
    <meta property="og:url" content="{base_url}/verify">
    <meta name="twitter:card" content="summary">
    <meta name="twitter:title" content="Genshin Roles - Link Account">
    <meta name="twitter:description" content="Link your Discord account with your Genshin Impact UID to automatically receive server roles based on your in-game progress.">
    <meta name="theme-color" content="#e8b44a">
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 580px; margin: 0 auto; padding: 32px 20px; background: #0e1525; color: #c8ccd4; min-height: 100vh; }}
        h1 {{ color: #e8b44a; font-size: 24px; margin-bottom: 4px; }}
        h2 {{ color: #fff; font-size: 17px; margin-bottom: 14px; }}
        p {{ line-height: 1.6; margin: 6px 0; font-size: 14px; }}
        a {{ color: #74b9ff; }}
        .subtitle {{ color: #7a8299; font-size: 14px; margin-bottom: 20px; }}
        .card {{ background: #161d2e; padding: 22px; border-radius: 10px; margin: 14px 0; border: 1px solid #1e2a3d; }}
        .btn {{ display: inline-flex; align-items: center; gap: 8px; padding: 10px 22px; color: #fff; text-decoration: none; border-radius: 6px; font-size: 14px; font-weight: 500; border: none; cursor: pointer; font-family: inherit; transition: background .15s; }}
        .btn-discord {{ background: #5865f2; }}
        .btn-discord:hover {{ background: #4752c4; }}
        .btn-primary {{ background: #3b82f6; }}
        .btn-primary:hover {{ background: #2563eb; }}
        .btn-success {{ background: #22c55e; }}
        .btn-success:hover {{ background: #16a34a; }}
        .btn-danger {{ background: transparent; color: #f87171; border: 1px solid #7f1d1d; font-size: 13px; padding: 8px 16px; }}
        .btn-danger:hover {{ background: #7f1d1d33; }}
        .btn-secondary {{ background: transparent; color: #94a3b8; border: 1px solid #334155; font-size: 13px; padding: 8px 16px; }}
        .btn-secondary:hover {{ background: #1e293b; }}
        .btn:disabled {{ opacity: 0.5; cursor: not-allowed; }}
        input {{ padding: 10px 14px; font-size: 15px; border-radius: 6px; border: 1px solid #2a3548; background: #0e1525; color: #e0e0e0; width: 100%; max-width: 240px; font-family: inherit; transition: border-color .15s; }}
        input:focus {{ outline: none; border-color: #3b82f6; }}
        .code-box {{ background: #0e1525; border: 2px dashed #e8b44a55; border-radius: 8px; padding: 16px; text-align: center; margin: 14px 0; }}
        .code {{ font-size: 30px; font-weight: 700; color: #e8b44a; letter-spacing: 4px; font-family: 'Courier New', monospace; user-select: all; }}
        .badge {{ display: inline-block; padding: 3px 10px; border-radius: 20px; font-size: 12px; font-weight: 500; }}
        .badge-ok {{ background: #052e16; color: #4ade80; border: 1px solid #14532d; }}
        .badge-wait {{ background: #1c1917; color: #fbbf24; border: 1px solid #422006; }}
        .msg {{ padding: 10px 14px; border-radius: 6px; margin: 12px 0; font-size: 13px; line-height: 1.5; }}
        .msg-error {{ background: #1c0a0a; color: #fca5a5; border: 1px solid #7f1d1d; }}
        .msg-success {{ background: #052e16; color: #86efac; border: 1px solid #14532d; }}
        .steps {{ margin: 12px 0; padding-left: 20px; }}
        .steps li {{ margin: 6px 0; font-size: 14px; line-height: 1.6; color: #94a3b8; }}
        .steps li strong {{ color: #e2e8f0; }}
        .note {{ font-size: 12px; color: #64748b; margin-top: 10px; font-style: italic; }}
        .trust-note {{ font-size: 13px; color: #94a3b8; background: #111827; border-left: 3px solid #3b82f6; padding: 10px 14px; border-radius: 0 6px 6px 0; margin: 10px 0; line-height: 1.6; }}
        .trust-note strong {{ color: #e2e8f0; }}
        .trust-note a {{ color: #74b9ff; }}
        .info-row {{ display: flex; align-items: center; gap: 8px; margin: 6px 0; font-size: 14px; }}
        .info-row .label {{ color: #64748b; min-width: 80px; }}
        .info-row .val {{ color: #e8b44a; font-weight: 600; }}
        .actions {{ display: flex; gap: 8px; margin-top: 16px; flex-wrap: wrap; }}
        .hidden {{ display: none !important; }}
        .divider {{ border: none; border-top: 1px solid #1e293b; margin: 16px 0; }}
    </style>
</head>
<body>
    <div style="display:flex; align-items:center; gap:10px; margin-bottom:4px;">
        <h1 style="margin:0;">Genshin Roles</h1>
        <span style="font-size:11px; color:#64748b; background:#1e293b; padding:2px 8px; border-radius:4px;">Powered by <a href="https://rolelogic.faizo.net" target="_blank" rel="noopener" style="color:#74b9ff; text-decoration:none;">RoleLogic</a></span>
    </div>
    <p class="subtitle">Link your Discord account with your Genshin Impact UID to automatically receive server roles based on your in-game progress.</p>

    <!-- Loading -->
    <div id="loading-section" class="card">
        <p style="color: #64748b;">Loading...</p>
    </div>

    <!-- Login -->
    <div id="login-section" class="card hidden">
        <h2>Step 1: Sign in with Discord</h2>
        <p>Sign in so we know which Discord account to assign roles to.</p>
        <p class="trust-note">We request the <strong>identify</strong> and <strong>guilds</strong> scopes — we cannot read your messages, join servers, or access anything else on your account.</p>
        <div class="actions">
            <a href="{login_url}" class="btn btn-discord">
                <svg width="20" height="15" viewBox="0 0 71 55" fill="white"><path d="M60.1 4.9A58.5 58.5 0 0045.4.2a.2.2 0 00-.2.1 40.8 40.8 0 00-1.8 3.7 54 54 0 00-16.2 0A37.3 37.3 0 0025.4.3a.2.2 0 00-.2-.1A58.4 58.4 0 0010.6 4.9a.2.2 0 00-.1.1C1.5 18 -.9 30.6.3 43a.2.2 0 00.1.2 58.7 58.7 0 0017.7 9 .2.2 0 00.3-.1 42 42 0 003.6-5.9.2.2 0 00-.1-.3 38.6 38.6 0 01-5.5-2.6.2.2 0 01 0-.4l1.1-.9a.2.2 0 01.2 0 41.9 41.9 0 0035.6 0 .2.2 0 01.2 0l1.1.9a.2.2 0 010 .3 36.3 36.3 0 01-5.5 2.7.2.2 0 00-.1.3 47.2 47.2 0 003.6 5.9.2.2 0 00.3.1A58.5 58.5 0 0070.3 43a.2.2 0 00.1-.2c1.4-14.7-2.4-27.5-10.2-38.8a.2.2 0 00-.1 0zM23.7 35.3c-3.4 0-6.1-3.1-6.1-6.8s2.7-6.9 6.1-6.9 6.2 3.1 6.1 6.9c0 3.7-2.7 6.8-6.1 6.8zm22.6 0c-3.4 0-6.1-3.1-6.1-6.8s2.7-6.9 6.1-6.9 6.2 3.1 6.1 6.9c0 3.7-2.7 6.8-6.1 6.8z"/></svg>
                Login with Discord
            </a>
        </div>
    </div>

    <!-- Linked -->
    <div id="linked-section" class="card hidden">
        <div style="display:flex; align-items:center; gap:10px; margin-bottom:14px;">
            <h2 style="margin:0;">Account Linked</h2>
            <span class="badge badge-ok">Verified</span>
        </div>
        <div class="info-row"><span class="label">Genshin</span> <span class="val" id="linked-uid"></span></div>
        <div class="info-row"><span class="label">Discord</span> <span class="val" id="linked-discord" style="color:#94a3b8;font-weight:400;font-size:13px;"></span></div>
        <p style="color:#4ade80; margin-top:12px; font-size:13px;">Your roles are assigned automatically based on your player data.</p>
        <p class="note">You can safely change your in-game signature back now.</p>
        <hr class="divider">
        <div class="actions">
            <button class="btn btn-danger" onclick="doUnlink()">Unlink Account</button>
        </div>
    </div>

    <!-- UID entry -->
    <div id="uid-section" class="card hidden">
        <h2>Step 2: Enter Your Genshin UID</h2>
        <p>Signed in as <span id="uid-discord" style="color:#74b9ff;"></span></p>
        <p style="margin-bottom:12px;">Your UID is the number at the bottom-right of the in-game screen, or in the Paimon Menu.</p>
        <div style="display:flex; gap:10px; align-items:center; flex-wrap:wrap;">
            <input type="text" id="uid-input" placeholder="e.g. 800000001" maxlength="10" inputmode="numeric" />
            <button class="btn btn-primary" id="start-btn" onclick="doStart()">Continue</button>
        </div>
        <p class="trust-note">We only read your public profile (Adventure Rank, achievements, etc.) using <a href="https://enka.network" target="_blank" rel="noopener">Enka.Network</a> — a widely used, read-only Genshin API. We have no access to your account, password, or any private data.</p>
    </div>

    <!-- Verify -->
    <div id="verify-section" class="card hidden">
        <div style="display:flex; align-items:center; gap:10px; margin-bottom:14px;">
            <h2 style="margin:0;">Step 3: Verify Ownership</h2>
            <span class="badge badge-wait">Pending</span>
        </div>
        <p class="trust-note" style="margin-bottom:12px;">To make sure this UID belongs to you, we need you to temporarily place a short code in your in-game signature. This is a standard verification method used by many Genshin community tools — it does not affect your account in any way.</p>
        <p>Set this code as your <strong style="color:#e2e8f0;">in-game signature</strong>:</p>
        <div class="code-box">
            <span class="code" id="verify-code"></span>
        </div>
        <p style="font-size:13px; color:#64748b;">Verifying: <span id="verify-uid" style="color:#74b9ff;"></span></p>
        <ol class="steps">
            <li>Open <strong>Genshin Impact</strong></li>
            <li>Go to <strong>Paimon Menu</strong> and tap your avatar (top-left)</li>
            <li>Tap the pencil icon next to your name and edit <strong>Signature</strong></li>
            <li>Paste or type the code above, then <strong>save</strong></li>
            <li>Click <strong>Quit Game</strong> (optional but recommended for a quick refresh)</li>
            <li>Come back here and click <strong>Verify Now</strong></li>
        </ol>
        <p class="note">You can remove the code from your signature right after verification.</p>
        <div class="actions">
            <button class="btn btn-success" id="verify-btn" onclick="doCheck()">Verify Now</button>
            <button class="btn btn-secondary" onclick="showSection('uid-section')">Change UID</button>
        </div>
    </div>

    <!-- Messages -->
    <div id="msg" class="hidden"></div>

    <noscript><p style="color:#f87171; margin-top:20px;">JavaScript is required.</p></noscript>

    <script>
    const API = '';

    async function api(method, path, body) {{
        const opts = {{ method, headers: {{}}, credentials: 'include' }};
        if (body) {{
            opts.headers['Content-Type'] = 'application/json';
            opts.body = JSON.stringify(body);
        }}
        const res = await fetch(API + path, opts);
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || 'Request failed');
        return data;
    }}

    function showSection(id) {{
        ['loading-section','login-section','linked-section','uid-section','verify-section'].forEach(s =>
            document.getElementById(s).classList.add('hidden')
        );
        document.getElementById(id).classList.remove('hidden');
    }}

    function showMsg(text, type) {{
        const el = document.getElementById('msg');
        el.className = 'msg msg-' + type;
        el.textContent = text;
        el.classList.remove('hidden');
        if (type === 'success') setTimeout(() => el.classList.add('hidden'), 6000);
    }}

    function clearMsg() {{ document.getElementById('msg').classList.add('hidden'); }}

    async function init() {{
        try {{
            const s = await api('GET', '/verify/status');
            currentName = s.display_name || '';
            if (s.linked) {{
                document.getElementById('linked-discord').textContent = s.display_name;
                document.getElementById('linked-uid').textContent = s.linked;
                showSection('linked-section');
            }} else if (s.pending_verification) {{
                document.getElementById('verify-code').textContent = s.pending_verification.code;
                document.getElementById('verify-uid').textContent = s.pending_verification.nickname
                    ? s.pending_verification.nickname + ' (AR ' + (s.pending_verification.level || '?') + ') - ' + s.pending_verification.uid
                    : s.pending_verification.uid;
                document.getElementById('uid-discord').textContent = s.display_name;
                showSection('verify-section');
            }} else {{
                document.getElementById('uid-discord').textContent = s.display_name;
                showSection('uid-section');
            }}
        }} catch (e) {{
            showSection('login-section');
        }}
    }}

    async function doStart() {{
        clearMsg();
        const uid = document.getElementById('uid-input').value.trim();
        if (!uid) return showMsg('Please enter your UID.', 'error');
        if (!/^\d{{9,10}}$/.test(uid)) return showMsg('UID must be 9 or 10 digits.', 'error');
        const btn = document.getElementById('start-btn');
        btn.disabled = true; btn.textContent = 'Looking up...';
        try {{
            const res = await api('POST', '/verify/start', {{ uid }});
            document.getElementById('verify-code').textContent = res.code;
            document.getElementById('verify-uid').textContent = res.nickname
                ? res.nickname + ' (AR ' + (res.level || '?') + ') - ' + res.uid
                : res.uid;
            showSection('verify-section');
        }} catch (e) {{ showMsg(e.message, 'error'); }}
        btn.disabled = false; btn.textContent = 'Continue';
    }}

    let currentName = '';
    let retryTimer = null;
    let hasRetried = false;

    async function doCheck() {{
        clearMsg();
        if (retryTimer) {{ clearInterval(retryTimer); retryTimer = null; }}
        const btn = document.getElementById('verify-btn');
        btn.disabled = true; btn.textContent = 'Checking...';
        try {{
            const res = await api('POST', '/verify/check');
            if (res.verified === false) {{
                if (hasRetried) {{
                    // Already retried once, show error and let user try manually
                    hasRetried = false;
                    showMsg('Code not found in your signature. Make sure you saved it in-game and try again.', 'error');
                    btn.disabled = false; btn.textContent = 'Verify Now';
                    return;
                }}
                // First attempt - auto-retry after TTL
                hasRetried = true;
                let ttl = Math.max(res.ttl || 5, 3);
                let remaining = ttl;
                btn.disabled = true;
                btn.textContent = 'Fetching ' + remaining + 's...';
                showMsg('Looking up your profile, please wait...', 'error');
                retryTimer = setInterval(() => {{
                    remaining--;
                    if (remaining <= 0) {{
                        clearInterval(retryTimer);
                        retryTimer = null;
                        doCheck();
                    }} else {{
                        btn.textContent = 'Fetching ' + remaining + 's...';
                    }}
                }}, 1000);
                return;
            }}
            hasRetried = false;
            let label = res.uid;
            if (res.nickname) label = res.nickname + ' (AR ' + (res.level || '?') + ') - ' + res.uid;
            document.getElementById('linked-uid').textContent = label;
            document.getElementById('linked-discord').textContent = currentName;
            showSection('linked-section');
            showMsg('Account linked successfully! You can now change your signature back.', 'success');
        }} catch (e) {{
            hasRetried = false;
            showMsg(e.message, 'error');
            btn.disabled = false; btn.textContent = 'Verify Now';
        }}
    }}

    async function doUnlink() {{
        clearMsg();
        if (!confirm('Unlink your account? You will lose all assigned roles.')) return;
        try {{
            await api('POST', '/verify/unlink');
            document.getElementById('uid-discord').textContent = currentName;
            showSection('uid-section');
            showMsg('Account unlinked.', 'success');
        }} catch (e) {{ showMsg(e.message, 'error'); }}
    }}

    document.getElementById('uid-input').addEventListener('keydown', e => {{
        if (e.key === 'Enter') doStart();
    }});

    init();
    </script>
</body>
</html>"##
    )
}

pub async fn verify_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        state.verify_html.clone(),
    )
}

pub async fn login(State(state): State<Arc<AppState>>) -> Response {
    let state_param: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let expires = chrono::Utc::now() + chrono::Duration::minutes(10);

    if let Err(e) = sqlx::query(
        "INSERT INTO oauth_states (state, expires_at) VALUES ($1, $2)",
    )
    .bind(&state_param)
    .bind(expires)
    .execute(&state.pool)
    .await
    {
        tracing::error!("Failed to store OAuth state: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response();
    }

    let url = DiscordOAuth::authorize_url(&state.config, &state_param);
    Redirect::temporary(&url).into_response()
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: String,
    pub error: Option<String>,
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<(CookieJar, Redirect), AppError> {
    // If user denied access, redirect back to verify page
    if query.error.is_some() || query.code.is_none() {
        return Ok((jar, Redirect::to(&format!("{}/verify", state.config.base_url))));
    }
    let code = query.code.unwrap();

    // Validate state (CSRF protection)
    let valid = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM oauth_states WHERE state = $1 AND expires_at > now())",
    )
    .bind(&query.state)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if !valid {
        return Err(AppError::BadRequest("Invalid or expired OAuth state".into()));
    }

    // Clean up used state
    sqlx::query("DELETE FROM oauth_states WHERE state = $1")
        .bind(&query.state)
        .execute(&state.pool)
        .await?;

    // Exchange code for token and get user info
    let oauth = DiscordOAuth::with_client(state.oauth_http.clone());
    let (access_token, refresh_token) = oauth.exchange_code(&state.config, &code).await?;
    let (discord_id, display_name) = oauth.get_user(&access_token).await?;

    // Store refresh token for periodic guild refresh (best-effort)
    if let Some(ref rt) = refresh_token {
        if let Err(e) = sqlx::query(
            "INSERT INTO discord_tokens (discord_id, refresh_token) VALUES ($1, $2) \
             ON CONFLICT (discord_id) DO UPDATE SET refresh_token = $2",
        )
        .bind(&discord_id)
        .bind(rt)
        .execute(&state.pool)
        .await
        {
            tracing::warn!(discord_id, "Failed to store refresh token: {e}");
        }
    }

    // Fetch and store guild memberships (best-effort, don't block login on failure)
    match oauth.get_user_guilds(&access_token).await {
        Ok(guilds) if !guilds.is_empty() => {
            let guild_ids: Vec<&str> = guilds.iter().map(|(id, _)| id.as_str()).collect();
            let guild_names: Vec<&str> = guilds.iter().map(|(_, name)| name.as_str()).collect();
            let mut tx = state.pool.begin().await?;
            sqlx::query("DELETE FROM user_guilds WHERE discord_id = $1")
                .bind(&discord_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO user_guilds (discord_id, guild_id, guild_name, updated_at) \
                 SELECT $1, UNNEST($2::text[]), UNNEST($3::text[]), now()",
            )
            .bind(&discord_id)
            .bind(&guild_ids)
            .bind(&guild_names)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            // Update guilds_refreshed_at timestamp
            let _ = sqlx::query(
                "UPDATE discord_tokens SET guilds_refreshed_at = now() WHERE discord_id = $1",
            )
            .bind(&discord_id)
            .execute(&state.pool)
            .await;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(discord_id, "Failed to fetch user guilds: {e}");
        }
    }

    // Create session cookie
    let session_value = discord_oauth::sign_session(&discord_id, &display_name, &state.config.session_secret);

    let cookie = Cookie::build((SESSION_COOKIE, session_value))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::hours(1));

    let jar = jar.add(cookie);

    Ok((jar, Redirect::to(&format!("{}/verify", state.config.base_url))))
}

pub async fn status(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<Json<Value>, AppError> {
    let (discord_id, display_name) = get_session(&jar, &state.config.session_secret)?;

    let account = sqlx::query_as::<_, (String,)>(
        "SELECT uid FROM linked_accounts WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .fetch_optional(&state.pool)
    .await?;

    // Check for pending verification
    let pending = sqlx::query_as::<_, (String, String)>(
        "SELECT uid, code FROM verification_sessions WHERE discord_id = $1 AND expires_at > now() ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&discord_id)
    .fetch_optional(&state.pool)
    .await?;

    let mut pending_info = pending.as_ref().map(|(uid, code)| json!({"uid": uid, "code": code}));

    // If pending, try to get player info from cache for display
    if let Some((uid, _)) = &pending {
        if let Ok(Some((player_info,))) = sqlx::query_as::<_, (Value,)>(
            "SELECT player_info FROM player_cache WHERE uid = $1",
        )
        .bind(uid)
        .fetch_optional(&state.pool)
        .await
        {
            if let Some(ref mut info) = pending_info {
                info["nickname"] = json!(player_info.get("nickname").and_then(|v| v.as_str()));
                info["level"] = json!(player_info.get("level").and_then(|v| v.as_i64()));
            }
        }
    }

    Ok(Json(json!({
        "discord_id": discord_id,
        "display_name": display_name,
        "linked": account.as_ref().map(|a| &a.0),
        "pending_verification": pending_info,
    })))
}

#[derive(Deserialize)]
pub struct StartBody {
    pub uid: String,
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(body): Json<StartBody>,
) -> Result<Json<Value>, AppError> {
    let (discord_id, _) = get_session(&jar, &state.config.session_secret)?;
    validate_uid(&body.uid)?;

    // Check if this Discord user already has a linked account
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT uid FROM linked_accounts WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .fetch_optional(&state.pool)
    .await?;

    if let Some(uid) = existing {
        return Err(AppError::BadRequest(format!(
            "You already have a linked account (UID: {uid}). Unlink it first."
        )));
    }

    // Check if UID is linked to another user
    let uid_taken = sqlx::query_scalar::<_, String>(
        "SELECT discord_id FROM linked_accounts WHERE uid = $1",
    )
    .bind(&body.uid)
    .fetch_optional(&state.pool)
    .await?;

    if uid_taken.is_some() {
        return Err(AppError::BadRequest(
            "This UID is already linked to another Discord account".into(),
        ));
    }

    // Delete any existing sessions for this user
    sqlx::query("DELETE FROM verification_sessions WHERE discord_id = $1")
        .bind(&discord_id)
        .execute(&state.pool)
        .await?;

    let code = generate_code();
    let expires = chrono::Utc::now() + chrono::Duration::minutes(15);

    sqlx::query(
        "INSERT INTO verification_sessions (discord_id, uid, code, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&discord_id)
    .bind(&body.uid)
    .bind(&code)
    .bind(expires)
    .execute(&state.pool)
    .await?;

    // Try to fetch player info for display
    let (nickname, level) = match state.enka_client.fetch_player_info(&body.uid).await {
        Ok(enka_result) => {
            let ttl = enka_result.ttl.max(60);
            let next_fetch = chrono::Utc::now() + chrono::Duration::seconds(ttl as i64);
            let _ = sqlx::query(
                "INSERT INTO player_cache (uid, player_info, region, enka_ttl, next_fetch_at, \
                 level, world_level, achievements, abyss_progress, fetter_count) \
                 VALUES ($1, $2, $3, $4, $5, \
                 COALESCE(($2->>'level')::int, 0), COALESCE(($2->>'worldLevel')::int, 0), \
                 COALESCE(($2->>'finishAchievementNum')::int, 0), \
                 COALESCE(($2->>'towerFloorIndex')::int, 0) * 10 + COALESCE(($2->>'towerLevelIndex')::int, 0), \
                 COALESCE(($2->>'fetterCount')::int, 0)) \
                 ON CONFLICT (uid) DO UPDATE SET \
                 player_info = $2, region = $3, enka_ttl = $4, \
                 fetched_at = now(), next_fetch_at = $5, fetch_failures = 0, \
                 level = COALESCE(($2->>'level')::int, 0), world_level = COALESCE(($2->>'worldLevel')::int, 0), \
                 achievements = COALESCE(($2->>'finishAchievementNum')::int, 0), \
                 abyss_progress = COALESCE(($2->>'towerFloorIndex')::int, 0) * 10 + COALESCE(($2->>'towerLevelIndex')::int, 0), \
                 fetter_count = COALESCE(($2->>'fetterCount')::int, 0)",
            )
            .bind(&body.uid)
            .bind(&enka_result.player_info)
            .bind(&enka_result.region)
            .bind(ttl)
            .bind(next_fetch)
            .execute(&state.pool)
            .await;

            let nickname = enka_result.player_info.get("nickname").and_then(|v| v.as_str()).map(String::from);
            let level = enka_result.player_info.get("level").and_then(|v| v.as_i64());
            (nickname, level)
        }
        Err(_) => (None, None),
    };

    Ok(Json(json!({
        "code": code,
        "uid": body.uid,
        "nickname": nickname,
        "level": level,
        "instructions": format!(
            "Set your in-game signature to include: {}  Then click Verify. The code expires in 15 minutes.",
            code
        )
    })))
}

pub async fn check(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<Json<Value>, AppError> {
    let (discord_id, display_name) = get_session(&jar, &state.config.session_secret)?;

    let session = sqlx::query_as::<_, (i64, String, String, i32)>(
        "SELECT id, uid, code, attempts FROM verification_sessions \
         WHERE discord_id = $1 AND expires_at > now() \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&discord_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound("No pending verification session. Start a new one.".into()))?;

    let (session_id, uid, code, attempts) = session;

    if attempts >= 10 {
        return Err(AppError::BadRequest(
            "Too many attempts. Please start a new verification.".into(),
        ));
    }

    // Increment attempts
    sqlx::query("UPDATE verification_sessions SET attempts = attempts + 1 WHERE id = $1")
        .bind(session_id)
        .execute(&state.pool)
        .await?;

    // Fetch from Enka API
    let enka_result = state.enka_client.fetch_player_info(&uid).await?;

    // Check signature contains the code
    let signature = enka_result
        .player_info
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !signature.contains(&code) {
        let ttl = enka_result.ttl.max(0);
        return Ok(Json(json!({
            "verified": false,
            "ttl": ttl,
            "signature": signature,
            "message": format!(
                "Code not found in your signature. Enka cache refreshes in ~{}s.",
                ttl
            )
        })));
    }

    // Verification succeeded - link account
    sqlx::query(
        "INSERT INTO linked_accounts (discord_id, uid, discord_name) VALUES ($1, $2, $3) \
         ON CONFLICT (discord_id) DO UPDATE SET uid = $2, linked_at = now(), discord_name = $3",
    )
    .bind(&discord_id)
    .bind(&uid)
    .bind(&display_name)
    .execute(&state.pool)
    .await?;

    // Cache the player data (avoids redundant Enka fetch)
    let ttl = enka_result.ttl.max(60);
    let next_fetch = chrono::Utc::now() + chrono::Duration::seconds(ttl as i64);

    sqlx::query(
        "INSERT INTO player_cache (uid, player_info, region, enka_ttl, next_fetch_at, \
         level, world_level, achievements, abyss_progress, fetter_count) \
         VALUES ($1, $2, $3, $4, $5, \
         COALESCE(($2->>'level')::int, 0), COALESCE(($2->>'worldLevel')::int, 0), \
         COALESCE(($2->>'finishAchievementNum')::int, 0), \
         COALESCE(($2->>'towerFloorIndex')::int, 0) * 10 + COALESCE(($2->>'towerLevelIndex')::int, 0), \
         COALESCE(($2->>'fetterCount')::int, 0)) \
         ON CONFLICT (uid) DO UPDATE SET \
         player_info = $2, region = $3, enka_ttl = $4, \
         fetched_at = now(), next_fetch_at = $5, fetch_failures = 0, \
         level = COALESCE(($2->>'level')::int, 0), world_level = COALESCE(($2->>'worldLevel')::int, 0), \
         achievements = COALESCE(($2->>'finishAchievementNum')::int, 0), \
         abyss_progress = COALESCE(($2->>'towerFloorIndex')::int, 0) * 10 + COALESCE(($2->>'towerLevelIndex')::int, 0), \
         fetter_count = COALESCE(($2->>'fetterCount')::int, 0)",
    )
    .bind(&uid)
    .bind(&enka_result.player_info)
    .bind(&enka_result.region)
    .bind(ttl)
    .bind(next_fetch)
    .execute(&state.pool)
    .await?;

    // Clean up verification session
    sqlx::query("DELETE FROM verification_sessions WHERE id = $1")
        .bind(session_id)
        .execute(&state.pool)
        .await?;

    // Trigger role sync for this user
    let _ = state
        .player_sync_tx
        .send(PlayerSyncEvent::AccountLinked {
            discord_id: discord_id.clone(),
        })
        .await;

    tracing::info!(discord_id, uid, "Account linked successfully");

    Ok(Json(json!({
        "verified": true,
        "uid": uid,
        "nickname": enka_result.player_info.get("nickname").and_then(|v| v.as_str()),
        "level": enka_result.player_info.get("level").and_then(|v| v.as_i64()),
    })))
}

pub async fn unlink(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<Json<Value>, AppError> {
    let (discord_id, _) = get_session(&jar, &state.config.session_secret)?;

    let account = sqlx::query_as::<_, (String,)>(
        "SELECT uid FROM linked_accounts WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound("No linked account found".into()))?;

    // Delete linked account
    sqlx::query("DELETE FROM linked_accounts WHERE discord_id = $1")
        .bind(&discord_id)
        .execute(&state.pool)
        .await?;

    // Trigger removal from all roles
    let _ = state
        .player_sync_tx
        .send(PlayerSyncEvent::AccountUnlinked {
            discord_id: discord_id.clone(),
        })
        .await;

    tracing::info!(discord_id, uid = account.0, "Account unlinked");

    Ok(Json(json!({"success": true})))
}
