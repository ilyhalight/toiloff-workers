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
    let session_messages = collect_client.get_session_messages(last_updated_at).await?;
    if session_messages.is_empty() {
        sheen::info!("No new session messages to send to API");
        return Ok(());
    }

    let latest_message: Option<&oc_collect::SessionMessageData> = session_messages
        .iter()
        .max_by_key(|data: &&oc_collect::SessionMessageData| data.message.time_updated);
    sheen::info!("Loaded session messages", count = session_messages.len(),);
    let api_sessions = internal_api::convert_to_api_session(&session_messages);
    let total_batches = api_sessions.len().div_ceil(internal_api::PUSH_BATCH_SIZE);
    sheen::info!(
        "Trying to send compacted sessions to API...",
        count = api_sessions.len(),
        batches = total_batches,
    );

    let mut pushed_count = 0;
    for (i, batch) in api_sessions
        .chunks(internal_api::PUSH_BATCH_SIZE)
        .enumerate()
    {
        let response = match internal_api::push_session_data(batch).await {
            Ok(response) => response,
            Err(err) => {
                sheen::error!(
                    "Failed to push compacted sessions batch!",
                    batch = i + 1,
                    total_batches = total_batches,
                    err = err
                );
                anyhow::bail!("read upper ^")
            }
        };

        match response {
            internal_api::SessionResponse::Failed(err) => {
                sheen::error!(&err.error, batch = i + 1);
                anyhow::bail!("read upper ^")
            }
            internal_api::SessionResponse::Valid(data) => {
                pushed_count += data.count;
                sheen::info!(
                    "Pushed batch",
                    batch = i + 1,
                    total_batches = total_batches,
                    count = data.count
                );
            }
        }
    }

    sheen::info!("Pushed compacted sessions to API!", count = pushed_count);
    if let Some(message_data) = latest_message {
        let new_updated_at = message_data.message.time_updated;
        cache::save_last_updated(new_updated_at);
        sheen::info!(
            "Latest session message time_updated!",
            new_updated_at = new_updated_at
        );
    }

    Ok(())
}
