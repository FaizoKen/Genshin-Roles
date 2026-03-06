use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Enka API error: {0}")]
    Enka(#[from] EnkaError),

    #[error("RoleLogic API error: {0}")]
    RoleLogic(String),

    #[error("Role link user limit reached ({limit})")]
    UserLimitReached { limit: usize },

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum EnkaError {
    #[error("Bad UID format")]
    BadUid,
    #[error("UID not found")]
    NotFound,
    #[error("Game maintenance")]
    Maintenance,
    #[error("Rate limited")]
    RateLimited,
    #[error("Server error: {0}")]
    Server(u16),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(e) => {
                tracing::error!("Database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::Enka(EnkaError::BadUid) => {
                (StatusCode::BAD_REQUEST, "Invalid UID. Please check and try again.")
            }
            AppError::Enka(EnkaError::NotFound) => {
                (StatusCode::NOT_FOUND, "Player not found. Make sure your UID is correct and your profile is public.")
            }
            AppError::Enka(EnkaError::RateLimited) => {
                (StatusCode::TOO_MANY_REQUESTS, "Too many requests. Please wait a moment and try again.")
            }
            AppError::Enka(EnkaError::Maintenance) => {
                (StatusCode::SERVICE_UNAVAILABLE, "Game data is temporarily unavailable (maintenance). Please try again later.")
            }
            AppError::Enka(e) => {
                tracing::error!("Enka API error: {e}");
                (StatusCode::BAD_GATEWAY, "Failed to fetch player data. Please try again later.")
            }
            AppError::RoleLogic(e) => {
                tracing::error!("RoleLogic API error: {e}");
                (StatusCode::BAD_GATEWAY, "Failed to sync roles")
            }
            AppError::UserLimitReached { limit } => {
                tracing::warn!("Role link user limit reached: {limit}");
                (StatusCode::FORBIDDEN, "Role link user limit reached")
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid or missing authorization"),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            AppError::VerificationFailed(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.as_str()),
            AppError::Internal(e) => {
                tracing::error!("Internal error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
