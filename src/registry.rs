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

/// Retrieves a list of all plugins and their descriptions.
pub async fn plugins(registry: Option<&str>) -> Result<Vec<PluginName>> {
    let mut path: Url = registry.unwrap_or(MEMFLOW_REGISTRY).parse().unwrap();
    path.set_path("plugins");

    let response = reqwest::get(path)
        .await?
        .json::<PluginsAllResponse>()
        .await?;

    Ok(response.plugins)
}

// Retrieves a list of all plugin versions
pub async fn download(plugin_name: &str) -> Result<()> {
    // TODO: unit tests
    let plugin: PluginUri = plugin_name.parse()?;
    println!("registry {}", plugin.registry());
    println!("name {}", plugin.name());
    println!("version {}", plugin.version());

    let mut path: Url = plugin.registry().parse().unwrap();
    path.set_path(&format!("plugins/{}", plugin.name()));

    // setup filters
    {
        let mut query = path.query_pairs_mut();
        if plugin.version() != "latest" {
            query.append_pair("version", plugin.version());
        }
        query.append_pair("memflow_plugin_version", "1"); // TODO:

        // file_type
        #[cfg(target_os = "windows")]
        query.append_pair("file_type", "pe");
        #[cfg(target_os = "linux")]
        query.append_pair("file_type", "elf");
        #[cfg(target_os = "macos")]
        query.append_pair("file_type", "mach");

        #[cfg(target_arch = "x86_64")]
        query.append_pair("architecture", "x86_64");
        #[cfg(target_arch = "x86")]
        query.append_pair("architecture", "x86");
        #[cfg(target_arch = "aarch64")]
        query.append_pair("architecture", "arm64");
        #[cfg(target_arch = "arm")]
        query.append_pair("architecture", "arm");

        // limit to the latest entry
        query.append_pair("limit", "1");
    }

    let response = reqwest::get(path)
        .await?
        .json::<PluginsFindResponse>()
        .await?;

    println!("response: {:?}", response);

    // TODO: download

    Ok(())
}

/// Parses a plugin string into it's path components
///
/// # Supported plugin path formats:
///
/// `coredump` - will just pull latest
/// `coredump:latest` - will also pull latest
/// `coredump:0.2.0` - will pull the newest binary with this specific version
/// `memflow.registry.io/coredump` - pulls from another registry
struct PluginUri {
    registry: Option<String>,
    name: String,
    version: Option<String>,
}

impl PluginUri {
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

impl FromStr for PluginUri {
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

        Ok(PluginUri {
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
        let path: PluginUri = "coredump".parse().unwrap();
        assert_eq!(path.registry(), MEMFLOW_REGISTRY);
        assert_eq!(path.name(), "coredump");
        assert_eq!(path.version(), "latest");
    }

    #[test]
    pub fn plugin_path_with_version() {
        let path: PluginUri = "coredump:0.2.0".parse().unwrap();
        assert_eq!(path.registry(), MEMFLOW_REGISTRY);
        assert_eq!(path.name(), "coredump");
        assert_eq!(path.version(), "0.2.0");
    }

    #[test]
    pub fn plugin_path_with_registry() {
        let path: PluginUri = "registry.memflow.xyz/coredump:0.2.0".parse().unwrap();
        assert_eq!(path.registry(), "registry.memflow.xyz");
        assert_eq!(path.name(), "coredump");
        assert_eq!(path.version(), "0.2.0");
    }

    #[test]
    pub fn plugin_path_invalid_path() {
        let path: Result<PluginUri> = "registry.memflow.xyz/coredump/test1234".parse();
        assert!(path.is_err())
    }

    #[test]
    pub fn plugin_path_invalid_version() {
        let path: Result<PluginUri> = "test1234:0.2.0:1.0.0".parse();
        assert!(path.is_err())
    }
}
