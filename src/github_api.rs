use crate::util;

use std::fs;

use log::{error, info};

use serde::{Deserialize, Serialize};

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
pub struct Commit {
    pub sha: String,
}

// TODO: filter for archicture
#[allow(unused)]
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn find_platform_asset(release: &Release) -> Option<&Asset> {
    release.assets.iter().find(|a| a.name.contains("linux"))
}

#[cfg(target_os = "windows")]
fn find_platform_asset<'a>(release: &'a Release) -> Option<&'a Asset> {
    release.assets.iter().find(|a| a.name.contains("windows"))
}

#[cfg(target_os = "macos")]
fn find_platform_asset<'a>(release: &'a Release) -> Option<&'a Asset> {
    release.assets.iter().find(|a| a.name.contains("macos"))
}

/// Downloads a release from the specified repository.
#[allow(unused)]
pub fn download_repository_release_latest(group: &str, repository: &str) -> Result<String, String> {
    let releases: Vec<Release> = util::http_get_json(&format!(
        "https://api.github.com/repos/{}/{}/releases",
        group, repository
    ))?;

    match releases.iter().find(|r| !r.draft && !r.prerelease) {
        Some(release) => {
            info!(
                "latest stable release: {} (tag: {})",
                release.name, release.tag_name
            );
            match find_platform_asset(release) {
                Some(asset) => {
                    info!("valid binary found for current platform: {}", asset.name);
                    let out_file = util::tmp_file(&asset.name);
                    download_file(&asset.browser_download_url, &out_file)?;
                    Ok(out_file)
                },
                None => {
                    Err(format!("unable to find appropiate binary for the current platform for release {}/{}/{}", group, repository, release.tag_name))
                }
            }
        }
        None => Err(format!(
            "unable to find a release for {}/{}",
            group, repository
        )),
    }
}

pub fn get_branch(url: &str, branch: &str) -> Result<Branch, &'static str> {
    let url = format!(
        "{}/branches/{}",
        url.replace("github.com", "api.github.com/repos"),
        branch
    );

    info!("Getting branch from {}", url);

    util::http_get_json(&url)
}

/// Downloads the given url to the destination file
#[allow(unused)]
fn download_file(url: &str, file: &str) -> Result<(), String> {
    info!("download file from '{}' to '{}'", url, file);
    let bytes = match util::http_download_file(url) {
        Ok(b) => b,
        Err(err) => {
            error!("{}", err);
            return Err(err.into());
        }
    };

    match fs::write(file, bytes) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("{}", err);
            Err(err.to_string())
        }
    }
}

pub fn download_raw(repo: &str, branch: &str, path: &str) -> Result<Vec<u8>, &'static str> {
    let url = format!(
        "{}/{}/{}",
        repo.replace("github.com", "raw.githubusercontent.com"),
        branch,
        path
    );
    info!("download raw from '{}'", url);

    util::http_download_file(&url)
}

pub fn download_release_artifact(
    repo: &str,
    release_tag: &str,
    artifact: &str,
) -> Result<Vec<u8>, &'static str> {
    let url = format!("{}/releases/download/{}/{}", repo, release_tag, artifact);
    info!("download artifact from '{}'", url);

    util::http_download_file(&url)
}
