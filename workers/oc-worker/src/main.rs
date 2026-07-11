mod api;
mod cache;

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
        sheen::info!("No new sessions to send to API.");
        return Ok(());
    }

    let latest_session = sessions.iter().max_by_key(|data| data.session.time_updated);
    sheen::info!("Trying to send sessions to API...", count = sessions.len(),);
    let api_sessions = api::convert_to_api_session(&sessions);
    let response = api::push_session_data(api_sessions).await?;
    sheen::info!("Pushed sessions to API!", count = response);
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
