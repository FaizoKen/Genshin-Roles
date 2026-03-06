use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::error::EnkaError;

pub struct EnkaResponse {
    pub player_info: serde_json::Value,
    pub region: Option<String>,
    pub ttl: i32,
}

pub struct EnkaClient {
    http: reqwest::Client,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
}

impl EnkaClient {
    pub fn new(user_agent: &str) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        // ~1.5 requests per second (3 per 2 seconds)
        let quota = Quota::per_second(NonZeroU32::new(2).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self { http, rate_limiter }
    }

    pub async fn wait_for_permit(&self) {
        self.rate_limiter.until_ready().await;
    }

    pub async fn fetch_player_info(&self, uid: &str) -> Result<EnkaResponse, EnkaError> {
        let url = format!("https://enka.network/api/uid/{uid}/?info");

        let resp = self.http.get(&url).send().await?;

        match resp.status().as_u16() {
            200 => {}
            400 => return Err(EnkaError::BadUid),
            404 => return Err(EnkaError::NotFound),
            424 => return Err(EnkaError::Maintenance),
            429 => return Err(EnkaError::RateLimited),
            code => return Err(EnkaError::Server(code)),
        }

        let body: serde_json::Value = resp.json().await?;

        let player_info = body
            .get("playerInfo")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default()));

        let region = body.get("region").and_then(|v| v.as_str()).map(String::from);
        let ttl = body.get("ttl").and_then(|v| v.as_i64()).unwrap_or(60) as i32;

        Ok(EnkaResponse {
            player_info,
            region,
            ttl,
        })
    }
}
