use std::sync::LazyLock;

use chrono::{Datelike, Utc};
use reqwest::{
    Client,
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

static REQ_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());
static GH_CLASSIC_TOKEN: LazyLock<String> = LazyLock::new(|| {
    let classic_token = std::env::var("GH_CLASSIC_TOKEN");
    match classic_token {
        Ok(val) => format!("Bearer {val}"),
        Err(_) => {
            sheen::error!("Please provide GH_CLASSIC_TOKEN environment variable");
            panic!("read upper ^")
        }
    }
});

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributionsCollection {
    #[serde(rename = "totalCommitContributions")]
    total_commit_contributions: u32,
    #[serde(rename = "contributionYears")]
    contribution_years: Vec<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewerWithContribs {
    pub login: String,
    #[serde(rename = "contributionsCollection")]
    pub contributions_collection: ContributionsCollection,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageInfo {
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
    #[serde(rename = "startCursor")]
    pub start_cursor: Option<String>,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "hasPreviousPage")]
    pub has_previous_page: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryOwner {
    pub login: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryNode {
    pub name: String,
    #[serde(rename = "stargazerCount")]
    pub stargazer_count: u32,
    pub owner: RepositoryOwner,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repositories {
    pub nodes: Vec<RepositoryNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewerWithRepos {
    pub login: String,
    pub repositories: Repositories,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewerData<T> {
    pub viewer: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GraphQLResponse<T> {
    Failed(ErrorResponse),
    Valid(SuccessResponse<T>),
}

pub type ViewerCommitsResponse = GraphQLResponse<ViewerData<ViewerWithContribs>>;
pub type ViewerReposResponse = GraphQLResponse<ViewerData<ViewerWithRepos>>;

pub fn get_contributions_query_part(from: String, to: String) -> String {
    format!(
        r##"contributionsCollection(from: "{from}", to: "{to}")
    {{
        totalCommitContributions
        contributionYears
    }}"##
    )
}

pub fn get_repositories_query_part(after: Option<String>) -> String {
    let after_value = match after {
        Some(val) => format!(r##""{val}""##),
        None => "null".to_string(),
    };
    format!(
        r##"repositories(first: 100, after: {after_value}, affiliations: [OWNER], isFork: false, privacy: PUBLIC)
    {{
        nodes {{
            name
            stargazerCount
            owner {{
                login
            }}
        }}
        pageInfo {{
            endCursor
            startCursor
            hasNextPage
            hasPreviousPage
        }}
    }}"##
    )
}

pub fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_str("application/vnd.github+json").unwrap(),
    );

    headers.insert(USER_AGENT, HeaderValue::from_str("gh-worker").unwrap());

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&GH_CLASSIC_TOKEN).unwrap(),
    );

    headers
}

pub async fn request_graphql<T: for<'de> Deserialize<'de>>(
    graphql_query: &String,
) -> Result<T, reqwest::Error> {
    let headers = get_headers();
    let request_body = json!({
        "query": graphql_query
    });

    let data = REQ_CLIENT
        .post("https://api.github.com/graphql")
        .headers(headers)
        .json(&request_body)
        .send()
        .await?
        .json::<T>()
        .await?;

    Ok(data)
}

pub fn iso8601_by_year(year: &u16) -> String {
    format!("{year}-01-01T00:00:01Z")
}

pub async fn get_commits_by_year(year: &u16) -> Result<ViewerCommitsResponse, reqwest::Error> {
    let from = iso8601_by_year(year);
    let next_year = year + 1;
    let to = iso8601_by_year(&next_year);
    let contribs_query = get_contributions_query_part(from, to);
    let graphql_query = format!(
        r##"query {{
        viewer {{
            login
            {contribs_query}
        }}
    }}"##
    );

    request_graphql::<ViewerCommitsResponse>(&graphql_query).await
}

pub async fn get_all_viewer_commits() -> anyhow::Result<u32> {
    let mut commits: u32 = 0;
    let current_year = Utc::now().year() as u16;
    sheen::info!("get commits by", year = &current_year);
    let initial_result = get_commits_by_year(&current_year).await;
    let Ok(initial_response) = initial_result else {
        sheen::error!("Failed to get initial data for all commits");
        anyhow::bail!("read upper ^")
    };

    let initial_data = match initial_response {
        GraphQLResponse::Valid(result) => result.data.viewer,
        GraphQLResponse::Failed(msg) => {
            sheen::error!("Failed to get initial data for all commits", msg = msg);
            anyhow::bail!("read upper ^")
        }
    };

    let required_years: Vec<u16> = initial_data
        .contributions_collection
        .contribution_years
        .iter()
        .filter(|year: &&u16| year != &&current_year)
        .map(|year| year.clone())
        .collect();
    commits += initial_data
        .contributions_collection
        .total_commit_contributions;
    for year in required_years {
        sheen::info!("get commits by", year = &year);
        let year_result = get_commits_by_year(&year).await;
        let Ok(year_response) = year_result else {
            sheen::error!("Failed to get commits by year. Skipped...", year = year);
            continue;
        };

        let year_commits = match year_response {
            GraphQLResponse::Valid(result) => {
                result
                    .data
                    .viewer
                    .contributions_collection
                    .total_commit_contributions
            }
            GraphQLResponse::Failed(msg) => {
                sheen::error!(
                    "Failed to get commits by year. Skipped",
                    year = year,
                    msg = msg
                );
                continue;
            }
        };

        commits += year_commits;
    }

    Ok(commits)
}

pub async fn get_viewer_repos(
    repos_after: Option<String>,
) -> Result<ViewerReposResponse, reqwest::Error> {
    let repos_query = get_repositories_query_part(repos_after);
    let graphql_query = format!(
        r##"query {{
        viewer {{
            login
            {repos_query}
        }}
    }}"##
    );

    request_graphql::<ViewerReposResponse>(&graphql_query).await
}

pub fn filter_repos_only_by_viewer(
    repos: Vec<RepositoryNode>,
    viewer: &String,
) -> Vec<RepositoryNode> {
    repos
        .into_iter()
        .filter(|node| &node.owner.login == viewer)
        .collect()
}

pub async fn get_all_viewer_stars() -> anyhow::Result<u32> {
    let repos_result = get_viewer_repos(None).await;
    let Ok(repos_response) = repos_result else {
        sheen::error!(
            "Failed to get initial data of viewer repos",
            err = repos_result.err()
        );
        anyhow::bail!("read upper ^")
    };

    let initial_data = match repos_response {
        GraphQLResponse::Valid(result) => result.data.viewer,
        GraphQLResponse::Failed(msg) => {
            sheen::error!("Failed to get initial data of viewer repos", msg = msg);
            anyhow::bail!("read upper ^")
        }
    };

    let viewer_login = initial_data.login;
    let mut repos = filter_repos_only_by_viewer(initial_data.repositories.nodes, &viewer_login);
    let mut repos_page = initial_data.repositories.page_info;
    while repos_page.has_next_page {
        sheen::info!("Checking next page for get extra user repos data",);
        let page_result = get_viewer_repos(repos_page.end_cursor.clone()).await;
        let Ok(page_response) = page_result else {
            sheen::error!("Failed to get next pages for user repos. Skipped...");
            break;
        };

        let page_data = match page_response {
            GraphQLResponse::Valid(result) => result.data.viewer,
            GraphQLResponse::Failed(msg) => {
                sheen::error!(
                    "Failed to get next pages for user repos. Skipped",
                    msg = msg
                );
                continue;
            }
        };

        let page_repos = filter_repos_only_by_viewer(page_data.repositories.nodes, &viewer_login);
        repos = repos.into_iter().chain(page_repos).collect();
        repos_page = page_data.repositories.page_info;
    }

    let stars: u32 = repos.into_iter().map(|repo| repo.stargazer_count).sum();
    Ok(stars)
}
