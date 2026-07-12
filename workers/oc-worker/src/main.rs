mod cache;
mod internal_api;

use dotenvy::dotenv;
use oc_collect::CollectClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    sheen::init();
    let last_updated_at = cache::get_last_updated();

    let mut collect_client = CollectClient::new();
    collect_client.open_pool().await?;
    let sessions = collect_client.get_usage_sessions(last_updated_at).await?;
    if sessions.is_empty() {
        sheen::info!("No new sessions to send to API");
        return Ok(());
    }

    let latest_session = sessions.iter().max_by_key(|data| data.session.time_updated);
    sheen::info!("Trying to send sessions to API...", count = sessions.len(),);
    let api_sessions = internal_api::convert_to_api_session(&sessions);
    let response = internal_api::push_session_data(api_sessions).await;
    if !response.is_ok() {
        sheen::error!("Failed to push session data!", err = response.err());
        anyhow::bail!("read upper ^")
    }

    let response_data = match response.unwrap() {
        internal_api::SessionResponse::Failed(err) => {
            sheen::error!(&err.error);
            anyhow::bail!("read upper ^")
        }
        internal_api::SessionResponse::Valid(data) => data,
    };

    sheen::info!("Pushed sessions to API!", count = response_data.count);
    if let Some(session_data) = latest_session {
        let new_updated_at = session_data.session.time_updated;
        cache::save_last_updated(new_updated_at);
        sheen::info!(
            "Latest session time_updated!",
            new_updated_at = new_updated_at
        );
    }

    Ok(())
}
