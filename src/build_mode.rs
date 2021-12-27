use crate::database::{load_database, DatabaseEntry, EntryType};
use crate::github_api;
use crate::scripting;
use crate::util;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use log::{info, warn};

use crate::package::*;

use crate::Result;

pub fn build(
    name: &str,
    path: &str,
    script: Option<&str>,
    ty: &str,
    unsafe_commands: bool,
    system_wide: bool,
    nocopy: bool,
) -> Result<()> {
    let ty = match ty {
        "core_plugin" | "core" => PackageType::CorePlugin,
        "utility" | "util" => PackageType::Utility,
        "library" | "lib" => PackageType::Library,
        "daemon_plugin" | "daemon" => PackageType::DaemonPlugin,
        _ => return Err("Invalid type".into()),
    };

    let package = Package {
        name: name.into(),
        repo_root_url: path.into(),
        install_script_path: script.map(<_>::into),
        unsafe_commands,
        ty,
        dev_branch: None,
        stable_branch: None,
        platforms: None,
    };

    let opts = PackageOpts {
        is_local: true,
        nocopy,
        system_wide,
    };

    package.install_local(&opts)?;

    Ok(())
}
