use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod db;
mod error;
mod models;
mod routes;
mod schema;
mod services;
mod tasks;

use services::enka::EnkaClient;
use services::rolelogic::RoleLogicClient;
use services::sync::SyncEvent;

pub struct AppState {
    pub pool: PgPool,
    pub config: config::AppConfig,
    pub sync_tx: mpsc::Sender<SyncEvent>,
    pub enka_client: EnkaClient,
    pub rl_client: RoleLogicClient,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "genshin_roles=info,tower_http=info".into()),
        )
        .init();

    let app_config = config::AppConfig::from_env();
    let listen_addr = app_config.listen_addr.clone();

    let pool = db::create_pool(&app_config.database_url).await;
    db::run_migrations(&pool).await;
    tracing::info!("Database connected and migrations applied");

    let (sync_tx, sync_rx) = mpsc::channel::<SyncEvent>(256);

    let enka_client = EnkaClient::new(&app_config.enka_user_agent);
    let rl_client = RoleLogicClient::new();

    let state = Arc::new(AppState {
        pool,
        config: app_config,
        sync_tx,
        enka_client,
        rl_client,
    });

    tokio::spawn(tasks::refresh_worker::run(Arc::clone(&state)));
    tokio::spawn(tasks::sync_worker::run(sync_rx, Arc::clone(&state)));
    tokio::spawn(tasks::cleanup_expired(Arc::clone(&state)));

    let app = Router::new()
        // Plugin endpoints (called by RoleLogic)
        .route("/register", post(routes::plugin::register))
        .route("/config", get(routes::plugin::get_config))
        .route("/config", post(routes::plugin::post_config))
        .route("/config", delete(routes::plugin::delete_config))
        // Verification endpoints (user-facing)
        .route("/verify", get(routes::verification::verify_page))
        .route("/verify/login", get(routes::verification::login))
        .route("/verify/callback", get(routes::verification::callback))
        .route("/verify/status", get(routes::verification::status))
        .route("/verify/start", post(routes::verification::start))
        .route("/verify/check", post(routes::verification::check))
        .route("/verify/unlink", post(routes::verification::unlink))
        // Health
        .route("/health", get(routes::health::health))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    tracing::info!("Server starting on {listen_addr}");

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("Failed to bind listener");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
