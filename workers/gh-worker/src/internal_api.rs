use std::{str::FromStr, sync::LazyLock};

use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};

static API_BASE_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
});
static API_SERVICE_TOKEN: LazyLock<String> = LazyLock::new(|| {
    let service_token = std::env::var("API_SERVICE_TOKEN");
    match service_token {
        Ok(val) => val,
        Err(_) => {
            sheen::error!("Please provide API_SERVICE_TOKEN environment variable");
            panic!("read upper ^")
        }
    }
});

#[derive(Debug, Serialize, Deserialize)]
pub struct StatSnapshot {
    pub commits: u32,
    pub stars: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SnapshotResponse {
    Failed(APIErrorResponse),
    Valid(StatSnapshot),
}

pub fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_str("x-service-token").unwrap(),
        HeaderValue::from_str(&API_SERVICE_TOKEN).unwrap(),
    );
    headers
}

pub async fn create_stat_snapshot(
    snapshot: StatSnapshot,
) -> Result<SnapshotResponse, reqwest::Error> {
    let headers = get_headers();
    Client::new()
        .post(&format!("{}/v1/stats/snapshot", API_BASE_URL.to_string()))
        .headers(headers)
        .json(&snapshot)
        .send()
        .await?
        .json::<SnapshotResponse>()
        .await
}
