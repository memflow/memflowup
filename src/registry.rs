use chrono::NaiveDateTime;
use reqwest::{IntoUrl, Url};
use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Clone, Deserialize)]
struct PluginsAllResponse {
    plugins: Vec<PluginName>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginName {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PluginsFindResponse {
    plugins: Vec<PluginEntry>,
    skip: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginEntry {
    pub digest: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub descriptor: PluginDescriptor,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginArchitecture {
    Unknown(u32),
    X86,
    X86_64,
    Arm,
    Arm64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginFileType {
    Pe,
    Elf,
    Mach,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PluginDescriptor {
    pub file_type: PluginFileType,
    pub architecture: PluginArchitecture,
    pub plugin_version: i32,
    pub name: String,
    pub version: String,
    pub description: String,
}

pub struct RegistryClient {
    url: Url,
}

impl RegistryClient {
    pub fn new<T: IntoUrl>(url: T) -> Self {
        Self {
            url: url.into_url().unwrap(),
        }
    }

    pub async fn plugins(&self) -> Result<Vec<PluginName>> {
        let mut path = self.url.clone();
        path.set_path("plugins");

        let response = reqwest::get(path)
            .await?
            .json::<PluginsAllResponse>()
            .await?;

        Ok(response.plugins)
    }
}
