use std::str::FromStr;

use chrono::NaiveDateTime;
use reqwest::{IntoUrl, Url};
use serde::Deserialize;

use crate::error::{Error, Result};

pub const MEMFLOW_REGISTRY: &str = "https://registry.memflow.io";

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

    /// Retrieves a list of all plugins and their descriptions.
    pub async fn plugins(&self) -> Result<Vec<PluginName>> {
        let mut path = self.url.clone();
        path.set_path("plugins");

        let response = reqwest::get(path)
            .await?
            .json::<PluginsAllResponse>()
            .await?;

        Ok(response.plugins)
    }

    // Retrieves a list of all plugin versions
    pub async fn download(&self, plugin_name: &str) -> Result<()> {
        // TODO: unit tests
        let plugin: PluginPath = plugin_name.parse()?;
        println!("registry {}", plugin.registry());
        println!("name {}", plugin.name());
        println!("version {}", plugin.version());

        let mut path = self.url.clone();
        //path.set_path(format!("plugins"));

        Ok(())
    }
}

/// Parses a plugin string into it's path components
///
/// # Supported plugin path formats:
///
/// `coredump` - will just pull latest
/// `coredump:latest` - will also pull latest
/// `coredump:0.2.0` - will pull the newest binary with this specific version
/// `memflow.registry.io/coredump` - pulls from another registry
struct PluginPath {
    registry: Option<String>,
    name: String,
    version: Option<String>,
}

impl PluginPath {
    #[inline]
    pub fn registry(&self) -> &str {
        self.registry.as_deref().unwrap_or(MEMFLOW_REGISTRY)
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn version(&self) -> &str {
        self.version.as_deref().unwrap_or("latest")
    }
}

impl FromStr for PluginPath {
    type Err = Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let path = s.split('/').collect::<Vec<_>>();
        let version = path
            .get(1)
            .unwrap_or_else(|| &path[0])
            .split(':')
            .collect::<Vec<_>>();
        if path.len() > 2 || version.len() > 2 {
            return Err(Error::Parse(
                "invalid plugin path. format should be {registry}/{plugin_name}:{plugin_version}"
                    .to_owned(),
            ));
        }

        Ok(PluginPath {
            registry: if path.len() > 1 {
                Some(path[0].to_owned())
            } else {
                None
            },
            name: version[0].to_owned(),
            version: version.get(1).map(|&s| s.to_owned()),
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn plugin_path_simple() {
        let path: PluginPath = "coredump".parse().unwrap();
        assert_eq!(path.registry(), MEMFLOW_REGISTRY);
        assert_eq!(path.name(), "coredump");
        assert_eq!(path.version(), "latest");
    }

    #[test]
    pub fn plugin_path_with_version() {
        let path: PluginPath = "coredump:0.2.0".parse().unwrap();
        assert_eq!(path.registry(), MEMFLOW_REGISTRY);
        assert_eq!(path.name(), "coredump");
        assert_eq!(path.version(), "0.2.0");
    }

    #[test]
    pub fn plugin_path_with_registry() {
        let path: PluginPath = "registry.memflow.xyz/coredump:0.2.0".parse().unwrap();
        assert_eq!(path.registry(), "registry.memflow.xyz");
        assert_eq!(path.name(), "coredump");
        assert_eq!(path.version(), "0.2.0");
    }

    #[test]
    pub fn plugin_path_invalid_path() {
        let path: Result<PluginPath> = "registry.memflow.xyz/coredump/test1234".parse();
        assert!(path.is_err())
    }

    #[test]
    pub fn plugin_path_invalid_version() {
        let path: Result<PluginPath> = "test1234:0.2.0:1.0.0".parse();
        assert!(path.is_err())
    }
}
