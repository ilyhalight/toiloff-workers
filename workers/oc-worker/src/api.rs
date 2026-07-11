use oc_collect::UsageSessionData;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct APISession {
    pub id: String,
    pub title: String,
    #[serde(rename = "tokensInput")]
    pub tokens_input: String,
    #[serde(rename = "tokensOutput")]
    pub tokens_output: String,
    #[serde(rename = "tokensCacheRead")]
    pub tokens_cache_read: String,
    pub model: String,
    #[serde(rename = "modelProvider")]
    pub model_provider: String,
    #[serde(rename = "modelVariant")]
    pub model_variant: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APISessionResponse {
    pub count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SessionResponse {
    Failed(APIErrorResponse),
    Valid(APISessionResponse),
}

pub fn convert_unix_to_iso(timestamp: i64) -> Option<String> {
    let datetime = chrono::DateTime::from_timestamp(timestamp / 1000, 0);
    match datetime {
        Some(dt) => {
            let time = dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            Some(time)
        }
        None => None,
    }
}

pub fn convert_to_api_session(session_data: &Vec<UsageSessionData>) -> Vec<APISession> {
    session_data
        .iter()
        .map(|session_item| -> APISession {
            let UsageSessionData { session, model } = session_item;
            let created_at = convert_unix_to_iso(session.time_created);
            let updated_at = convert_unix_to_iso(session.time_updated);

            APISession {
                id: session.id.clone(),
                title: session.title.clone(),
                tokens_input: session.tokens_input.to_string(),
                tokens_output: session.tokens_output.to_string(),
                tokens_cache_read: session.tokens_cache_read.to_string(),
                model: model.id.clone(),
                model_provider: model.provider_id.clone(),
                model_variant: model.variant.clone(),
                created_at,
                updated_at,
            }
        })
        .collect()
}

pub fn get_base_url() -> String {
    let base_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string());
    base_url
}

pub fn get_service_token() -> String {
    let service_key = std::env::var("API_SERVICE_TOKEN")
        .expect("Please provide API_SERVICE_TOKEN environment variable");
    service_key
}

pub async fn push_session_data(
    api_sessions: Vec<APISession>,
) -> Result<SessionResponse, reqwest::Error> {
    Client::new()
        .post(&format!("{}/v1/stats/upsert", get_base_url()))
        .header("x-service-token", get_service_token())
        .json(&api_sessions)
        .send()
        .await?
        .json::<SessionResponse>()
        .await
}
