use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub session_secret: String,
    pub enka_user_agent: String,
    pub base_url: String,
    pub listen_addr: String,
    /// Base URL of the Auth Gateway (without trailing slash, no `/auth` suffix).
    /// Used by the plugin to query guild membership/permission.
    /// In production this is usually the same origin as `BASE_URL`
    /// (e.g. `https://plugin-rolelogic.faizo.net`). For local dev, set this to the
    /// Auth Gateway's local listener (e.g. `http://localhost:8080`) so that the
    /// plugin talks to the local gateway instead of going back through Cloudflare.
    pub auth_gateway_url: String,
}

fn derive_origin(base_url: &str) -> String {
    if let Some(scheme_end) = base_url.find("://") {
        let after_scheme = scheme_end + 3;
        if let Some(path_slash) = base_url[after_scheme..].find('/') {
            return base_url[..after_scheme + path_slash].to_string();
        }
    }
    base_url.to_string()
}

impl AppConfig {
    pub fn from_env() -> Self {
        let base_url = env::var("BASE_URL").expect("BASE_URL must be set");
        let auth_gateway_url = env::var("AUTH_GATEWAY_URL")
            .ok()
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| derive_origin(&base_url));

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            session_secret: env::var("SESSION_SECRET").expect("SESSION_SECRET must be set"),
            enka_user_agent: env::var("ENKA_USER_AGENT")
                .unwrap_or_else(|_| "GenshinPlayerRole/1.0".to_string()),
            base_url,
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            auth_gateway_url,
        }
    }
}
