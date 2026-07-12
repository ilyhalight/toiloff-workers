mod github_api;
mod internal_api;

use dotenvy::dotenv;

use crate::internal_api::StatSnapshot;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    sheen::init();

    sheen::info!("Trying to get user commits from API...");
    let commits_result = github_api::get_all_viewer_commits().await;
    let Ok(commits) = commits_result else {
        anyhow::bail!("read upper ^")
    };
    sheen::info!("total", commits = commits);

    sheen::info!("Trying to get user stars from API...");
    let stars_result = github_api::get_all_viewer_stars().await;
    let Ok(stars) = stars_result else {
        anyhow::bail!("read upper ^")
    };
    sheen::info!("total", stars = stars);

    let snapshot = StatSnapshot { stars, commits };

    let stats_response = internal_api::create_stat_snapshot(snapshot).await;
    if !stats_response.is_ok() {
        sheen::error!(
            "Failed to create stat snapshot!",
            err = stats_response.err()
        );
        anyhow::bail!("read upper ^")
    }

    let response_data = match stats_response.unwrap() {
        internal_api::SnapshotResponse::Failed(err) => {
            sheen::error!(&err.error);
            anyhow::bail!("read upper ^")
        }
        internal_api::SnapshotResponse::Valid(data) => data,
    };

    sheen::info!("Pushed stat snapshot to API!", response = response_data);
    Ok(())
}
