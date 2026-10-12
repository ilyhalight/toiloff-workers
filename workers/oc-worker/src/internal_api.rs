use std::collections::HashMap;
use std::sync::LazyLock;

use oc_collect::SessionMessageData;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};

const COMPACT_WINDOW_MS: i64 = 60 * 60 * 1000;
pub const PUSH_BATCH_SIZE: usize = 1000;

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
#[serde(rename_all = "camelCase")]
pub struct APISession {
    pub title: String,
    pub tokens_input: String,
    pub tokens_output: String,
    pub tokens_cache_read: String,
    pub model: String,
    pub model_provider: String,
    pub model_variant: Option<String>,
    pub created_at: Option<String>,
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

type ModelKey<'a> = (&'a str, &'a str, Option<&'a str>);

#[derive(Default)]
struct SessionStats {
    max_updated_at: i64,
    total_cost: f64,
    total_input_tokens: i64,
    total_output_tokens: i64,
    total_cache_read_tokens: i64,
}

fn build_api_session(
    model_id: &str,
    model_provider: &str,
    model_variant: Option<&str>,
    window: &[&SessionMessageData],
) -> APISession {
    let stats = window.iter().fold(SessionStats::default(), |mut acc, m| {
        let msg = &m.message;
        acc.max_updated_at = acc.max_updated_at.max(msg.time_updated);
        acc.total_cost += msg.cost;
        acc.total_input_tokens += msg.tokens_input;
        acc.total_output_tokens += msg.tokens_output + msg.tokens_reasoning;
        acc.total_cache_read_tokens += msg.tokens_cache_read;
        acc
    });

    let created_at = convert_unix_to_iso(window[0].message.time_created);
    let updated_at = convert_unix_to_iso(stats.max_updated_at);

    let title = format!(
        "{} ({}) by {} - {}$",
        model_id,
        model_variant.unwrap_or(""),
        model_provider,
        stats.total_cost
    );

    APISession {
        title,
        tokens_input: stats.total_input_tokens.to_string(),
        tokens_output: stats.total_output_tokens.to_string(),
        tokens_cache_read: stats.total_cache_read_tokens.to_string(),
        model: model_id.to_string(),
        model_provider: model_provider.to_string(),
        model_variant: model_variant.map(str::to_string),
        created_at,
        updated_at,
    }
}

pub fn convert_to_api_session(messages_data: &[SessionMessageData]) -> Vec<APISession> {
    let mut by_model: HashMap<ModelKey, Vec<&SessionMessageData>> = HashMap::new();
    for data in messages_data {
        let model = &data.model;
        if model.id.is_empty() {
            continue;
        }

        by_model
            .entry((&model.id, &model.provider_id, model.variant.as_deref()))
            .or_default()
            .push(data);
    }

    let mut api_sessions: Vec<APISession> = Vec::new();
    for ((model_id, model_provider, model_variant), mut messages) in by_model {
        messages.sort_by_key(|m| m.message.time_created);

        let mut rest = messages.as_slice();
        while let Some(first) = rest.first() {
            let window_end = first.message.time_created + COMPACT_WINDOW_MS;
            let len = rest.partition_point(|m| m.message.time_created <= window_end);
            let (window, tail) = rest.split_at(len);

            api_sessions.push(build_api_session(
                model_id,
                model_provider,
                model_variant,
                window,
            ));
            rest = tail;
        }
    }

    api_sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    api_sessions
}

pub fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-service-token",
        HeaderValue::from_str(&API_SERVICE_TOKEN).unwrap(),
    );
    headers
}

pub async fn push_session_data(
    api_sessions: &[APISession],
) -> Result<SessionResponse, reqwest::Error> {
    let headers = get_headers();
    Client::new()
        .post(&format!("{}/v1/stats/upsert", API_BASE_URL.to_string()))
        .headers(headers)
        .json(&api_sessions)
        .send()
        .await?
        .json::<SessionResponse>()
        .await
}
