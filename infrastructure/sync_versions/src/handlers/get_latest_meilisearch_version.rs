use log::{debug, error, info};
use reqwest::blocking;
use serde::Deserialize;
use std::error;

#[derive(Debug, Deserialize)]
struct Tag {
    name: String,
}

const TAGS_URL: &str = "https://api.github.com/repos/meilisearch/meilisearch/tags";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Fetch the latest Meilisearch release tag from the GitHub API.
pub fn handle() -> Result<String, Box<dyn error::Error>> {
    let client = blocking::Client::new();

    debug!("Fetching tags from {}", TAGS_URL);
    let response: Vec<Tag> = client
        .get(TAGS_URL)
        .header("User-Agent", USER_AGENT)
        .send()?
        .json()?;

    debug!("Fetched {} tags", response.len());
    if response.is_empty() {
        error!("No tags found at {}", TAGS_URL);
        return Err(format!("Tag not found: {}", TAGS_URL).into());
    }

    let latest_tag = response[0].name.clone();
    info!("Latest tag: {latest_tag}");
    return Ok(latest_tag);
}
