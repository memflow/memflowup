use serde::*;

use crate::{database, scripting, util, Result};
use database::Branch;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use log::*;

const BUILTIN_INDEX: &str = include_str!("../index.json");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Package {
    pub name: String,
    pub ty: PackageType,
    pub platforms: Option<Vec<String>>,
    pub repo_root_url: String,
    pub stable_branch: Option<String>,
    pub dev_branch: Option<String>,
    #[serde(default)]
    pub unsafe_commands: bool,
    pub install_script_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackageOpts {
    pub is_local: bool,
    pub nocopy: bool,
    pub system_wide: bool,
}

impl PackageOpts {
    pub fn system_wide(system_wide: bool) -> Self {
        Self {
            system_wide,
            ..Default::default()
        }
    }
}

impl Package {
    pub fn install_source(&self, branch: Branch, opts: &PackageOpts) -> Result<()> {
        let (ty, artifacts) =
            scripting::execute_installer(self, opts, branch, "build_from_source")?;

        database::commit_entry(
            &self.name,
            database::DatabaseEntry { ty, artifacts },
            branch,
            opts.system_wide,
        )
    }

    pub fn install_local(&self, opts: &PackageOpts) -> Result<()> {
        let (ty, artifacts) =
            scripting::execute_installer(self, opts, Branch::Local, "build_local")?;

        database::commit_entry(
            &self.name,
            database::DatabaseEntry { ty, artifacts },
            Branch::Local,
            opts.system_wide,
        )
    }

    pub fn supported_by_platform(&self) -> bool {
        if let Some(platforms) = &self.platforms {
            for s in &[
                #[cfg(target_os = "linux")]
                "linux",
                #[cfg(unix)]
                "unix",
                #[cfg(target_os = "macos")]
                "macos",
                #[cfg(windows)]
                "windows",
            ] {
                if platforms.iter().any(|p| p == s) {
                    return true;
                }
            }

            false
        } else {
            true
        }
    }

    pub fn is_in_channel(&self, branch: Branch) -> bool {
        self.branch(branch).is_some()
    }

    pub fn branch(&self, branch: Branch) -> Option<&str> {
        match branch {
            Branch::Dev => self.dev_branch.as_deref(),
            Branch::Stable => self.stable_branch.as_deref(),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PackageType {
    CorePlugin,
    Utility,
    Library,
    DaemonPlugin,
}

impl PackageType {
    pub fn install_path(&self, system_wide: bool) -> PathBuf {
        match *self {
            PackageType::CorePlugin if system_wide => {
                if cfg!(unix) {
                    PathBuf::from("/").join("usr").join("lib").join("memflow")
                } else {
                    PathBuf::from("C:\\").join("Program Files").join("memflow")
                }
            }
            PackageType::CorePlugin => {
                if cfg!(unix) {
                    dirs::home_dir()
                        .unwrap()
                        .join(".local")
                        .join("lib")
                        .join("memflow")
                } else {
                    dirs::document_dir().unwrap().join("memflow")
                }
            }
            p => unreachable!("package type {:?} is unsupported!", p),
        }
    }
}

pub fn update_index(system_wide: bool) -> Result<()> {
    let mut path = util::config_dir(system_wide);
    path.push("index");
    path.push("default.json");

    if let Ok(metadata) = fs::metadata(&path) {
        if let Ok(modified) = metadata.modified() {
            match modified.elapsed() {
                Err(_) => return Ok(()),
                Ok(dur) => {
                    if dur.as_secs() <= 60 {
                        return Ok(());
                    }
                }
            }
        }
    }

    info!("Updating index:");

    match util::http_download_file(
        "https://raw.githubusercontent.com/memflow/memflowup/master/index.json",
    ) {
        Ok(bytes) => {
            util::create_dir_with_elevation(path.as_path().parent().unwrap(), system_wide)?;

            util::write_with_elevation(path, system_wide, |mut file| {
                file.write_all(&bytes).map_err(Into::into)
            })
        }
        Err(e) => {
            error!("Failed to download index: {}", e);
            Ok(())
        }
    }
}

pub fn load_packages(system_wide: bool) -> Result<Vec<Package>> {
    let mut ret = vec![];

    let mut path = util::config_dir(system_wide);

    path.push("index");

    path.push("custom");

    // First load user specified indices

    for p in fs::read_dir(&path).into_iter().flatten() {
        let p = p?.path();
        if p.extension().map(|s| s.to_str()).flatten() == Some("json") {
            let file = fs::File::open(p)?;
            let parsed: Vec<Package> = serde_json::from_reader(file)?;
            ret.extend(parsed);
        }
    }

    path.pop();

    // Then the default index

    path.push("default.json");

    if let Ok(file) = fs::File::open(path) {
        let parsed: Vec<Package> = serde_json::from_reader(file)?;
        ret.extend(parsed);
    }

    // Then the builtin index

    let parsed: Vec<Package> = serde_json::from_str(BUILTIN_INDEX)?;
    ret.extend(parsed);

    // And finally, dedup everything so user has no duplicate entries, and user's entries are the
    // ones preserved in case of duplicates.

    let mut found = std::collections::HashSet::new();

    ret.retain(|item| {
        if found.contains(&item.name) {
            false
        } else {
            found.insert(item.name.clone());
            true
        }
    });

    Ok(ret)
}
