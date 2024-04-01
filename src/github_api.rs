use reqwest::{Response, Url};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const USER_AGENT: &str = "memflowup 0.2.0";

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    pub url: String,
    pub assets_url: String,
    pub upload_url: String,
    pub html_url: String,
    pub id: u64,
    // author
    pub node_id: String,
    pub tag_name: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: String,
    pub published_at: String,
    pub assets: Vec<Asset>,
    pub tarball_url: String,
    pub zipball_url: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub url: String,
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub label: String,
    // uploader
    pub content_type: String,
    pub state: String,
    pub size: u64,
    pub download_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub commit: Commit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    #[serde(rename = "ref")]
    pub name: String,
    #[serde(rename = "object")]
    pub commit: Commit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
}

/// Resolves a specific branch from github
pub async fn branch(url: &str, branch: &str) -> Result<Branch> {
    if !url.contains("github.com") {
        return Err(Error::Http(
            "github api only works with github.com api".to_owned(),
        ));
    }

    let path: Url = format!(
        "{}/branches/{}",
        url.replace("github.com", "api.github.com/repos"),
        branch
    )
    .parse()
    .unwrap(); // TODO: parse error

    let client = reqwest::Client::new();
    let response = client
        .get(path)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let result = response.json::<Branch>().await?;
    Ok(result)
}

/// Resolves a specific tag from github
pub async fn tag(url: &str, tag: &str) -> Result<Tag> {
    if !url.contains("github.com") {
        return Err(Error::Http(
            "github api only works with github.com api".to_owned(),
        ));
    }

    let path: Url = format!(
        "{}/git/ref/tags/{}",
        url.replace("github.com", "api.github.com/repos"),
        tag
    )
    .parse()
    .unwrap(); // TODO: parse error

    let client = reqwest::Client::new();
    let response = client
        .get(path)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let result = response.json::<Tag>().await?;
    Ok(result)
}

/// Downloads the code for specific commit in the repository
pub async fn download_code_for_commit(url: &str, commit: &str) -> Result<Response> {
    if !url.contains("github.com") {
        return Err(Error::Http(
            "github api only works with github.com api".to_owned(),
        ));
    }

    let path: Url = format!("{}/archive/{}.zip", url, commit).parse().unwrap(); // TODO: parse error

    let client = reqwest::Client::new();
    let response = client
        .get(path)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    Ok(response)
}
