use serde::*;

use crate::{
    database::{self, load_database, DatabaseEntry, EntryType},
    scripting, util, Result,
};
use database::Branch;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use log::*;

const BUILTIN_INDEX: &str = include_str!("../index.json");

#[derive(Default, Copy, Clone, Debug)]
pub struct PackageLoadOpts {
    pub ignore_user: bool,
    pub ignore_upstream: bool,
    pub ignore_builtin: bool,
}

impl PackageLoadOpts {
    pub fn new(ignore_user: bool, ignore_upstream: bool, ignore_builtin: bool) -> Self {
        Self {
            ignore_user,
            ignore_upstream,
            ignore_builtin,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Package {
    pub name: String,
    pub ty: PackageType,
    pub platforms: Option<Vec<String>>,
    pub repo_root_url: String,
    pub stable_branch: Option<String>,
    pub stable_binary_tag: Option<String>,
    pub dev_branch: Option<String>,
    pub dev_binary_tag: Option<String>,
    #[serde(default)]
    pub unsafe_commands: bool,
    pub install_script_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackageOpts {
    pub is_local: bool,
    pub nocopy: bool,
    pub system_wide: bool,
    pub reinstall: bool,
    pub from_source: bool,
}

impl PackageOpts {
    pub fn base_opts(system_wide: bool, from_source: bool) -> Self {
        Self {
            system_wide,
            from_source,
            ..Default::default()
        }
    }
}

impl Package {
    pub fn install(&self, branch: Branch, opts: &PackageOpts) -> Result<()> {
        if opts.from_source {
            self.install_inner(branch, "build_from_source", opts)
        } else {
            self.install_inner(branch, "install", opts)
        }
    }

    fn install_inner(&self, branch: Branch, entrypoint: &str, opts: &PackageOpts) -> Result<()> {
        let (ty, artifacts) = scripting::execute_installer(self, opts, branch, entrypoint)?;

        database::commit_entry(
            &self.name,
            database::DatabaseEntry { ty, artifacts },
            branch,
            opts.system_wide,
        )
    }

    pub fn install_local(&self, opts: &PackageOpts) -> Result<()> {
        self.install_inner(Branch::Local, "build_local", opts)
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

    pub fn supports_install_mode(&self, branch: Branch, from_source: bool) -> bool {
        self.is_in_channel(branch) && (from_source || self.binary_release_tag(branch).is_some())
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

    pub fn binary_release_tag(&self, branch: Branch) -> Option<&str> {
        match branch {
            Branch::Dev => self.dev_binary_tag.as_deref(),
            Branch::Stable => self.stable_binary_tag.as_deref(),
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
                    PathBuf::from("/")
                        .join("usr")
                        .join("local")
                        .join("lib")
                        .join("memflow")
                } else {
                    let programfiles = std::env::var_os("PROGRAMFILES").unwrap();
                    PathBuf::from(programfiles).join("memflow")
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

pub fn load_packages(system_wide: bool, load_opts: PackageLoadOpts) -> Result<Vec<Package>> {
    let mut ret = vec![];

    let mut path = util::config_dir(system_wide);

    path.push("index");

    // First load user specified indices

    if !load_opts.ignore_user {
        path.push("custom");

        for p in fs::read_dir(&path).into_iter().flatten() {
            let p = p?.path();
            if p.extension().map(|s| s.to_str()).flatten() == Some("json") {
                let file = fs::File::open(p)?;
                let parsed: Vec<Package> = serde_json::from_reader(file)?;
                ret.extend(parsed);
            }
        }

        path.pop();
    }

    // Then the default index

    if !load_opts.ignore_upstream {
        path.push("default.json");

        if let Ok(file) = fs::File::open(path) {
            let parsed: Vec<Package> = serde_json::from_reader(file)?;
            ret.extend(parsed);
        }
    }

    // Then the builtin index

    if !load_opts.ignore_builtin {
        let parsed: Vec<Package> = serde_json::from_str(BUILTIN_INDEX)?;
        ret.extend(parsed);
    }

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

pub fn list(
    system_wide: bool,
    branch: Branch,
    from_source: bool,
    load_opts: PackageLoadOpts,
) -> Result<()> {
    if !load_opts.ignore_upstream {
        update_index(system_wide)?;
    }

    let packages = load_packages(system_wide, load_opts)?;

    let db = load_database(branch, system_wide)?;

    println!("Available packages in {} channel:", branch.filename());

    for (i, package) in packages
        .iter()
        .filter(|p| p.supports_install_mode(branch, from_source))
        .filter(|p| p.supported_by_platform())
        .enumerate()
    {
        print!("{}. {} - {:?}", i, package.name, package.ty);

        match db.get(&package.name) {
            Some(DatabaseEntry {
                ty: EntryType::GitSource(hash),
                ..
            }) => print!(
                " [installed: git {}]",
                hash.chars().take(6).collect::<String>()
            ),
            Some(DatabaseEntry {
                ty: EntryType::Binary(tag),
                ..
            }) => print!(" [installed: binary {}]", tag),
            Some(DatabaseEntry {
                ty: EntryType::LocalPath(tag),
                ..
            }) => print!(" [installed: path {}]", tag),
            Some(DatabaseEntry {
                ty: EntryType::Crates(version),
                ..
            }) => print!(" [installed: crates.io {}]", version),
            None => {}
        }

        println!();
    }

    Ok(())
}

pub fn update(system_wide: bool, dev: bool, load_opts: PackageLoadOpts) -> Result<()> {
    update_index(system_wide)?;

    let packages = load_packages(system_wide, load_opts)?;

    let branch = dev.into();

    let db = load_database(branch, system_wide)?;

    println!("Upgrading packages in {} channel:", branch.filename());

    for (i, package) in packages
        .iter()
        .filter(|p| p.is_in_channel(branch))
        .filter(|p| p.supported_by_platform())
        .filter(|p| db.get(&p.name).is_some())
        .enumerate()
    {
        println!("{}. {} - {:?}", i, package.name, package.ty);
    }
    println!();

    for (package, from_source) in packages
        .iter()
        .filter(|p| p.is_in_channel(branch))
        .filter(|p| p.supported_by_platform())
        .filter_map(|p| match db.get(&p.name) {
            Some(DatabaseEntry {
                ty: EntryType::Binary(_),
                ..
            }) => Some((p, false)),
            Some(_) => Some((p, true)),
            _ => None,
        })
    {
        println!("Upgrading {}:", package.name);
        package.install(branch, &PackageOpts::base_opts(system_wide, from_source))?;
        println!();
    }

    Ok(())
}
