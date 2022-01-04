use crate::database::{load_database, DatabaseEntry, EntryType};
use crate::util;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::process::{Command, Stdio};

use log::{info, warn};

use crate::package::*;

use crate::Result;

pub fn setup_mode() -> Result<()> {
    // 1. ensure rustup / cargo is installed in PATH
    ensure_rust()?;

    // 2. install memflowup in PATH

    // 3. install default set of connectors for the current platform
    install_modules()
}

fn ensure_rust() -> Result<()> {
    match which::which("cargo") {
        Ok(cargo_dir) => {
            info!("cargo found at {:?}", cargo_dir);
            Ok(())
        }
        Err(_) => {
            warn!("cargo not found");
            if !cfg!(windows)
                && util::user_input_boolean(
                    "do you want memflowup to install rust via rustup?",
                    true,
                )?
            {
                info!("cargo not found, installing via rustup");
                install_rust()
            } else {
                Err("rust/cargo not found. please install it manually.".into())
            }
        }
    }
}

// TODO: windows / mac support
/// Downloads and executes rustup or panics
fn install_rust() -> Result<()> {
    match which::which("rustup") {
        Ok(rustup_path) => {
            info!("rustup found at {:?}", rustup_path);
            install_rust_toolchain(rustup_path)
        }
        Err(_) if !cfg!(unix) => {
            warn!("rustup is not installed, trying to download");
            install_rustup().and_then(|_| {
                install_rust_toolchain(
                    which::which("rustup").expect("No rustup found after installing rustup!"),
                )
            })
        }
        _ => {
            warn!("rustup is not installed, setup manually!");
            Err("Please install rustup".into())
        }
    }
}

// TODO: windows / mac support
fn install_rust_toolchain<P: AsRef<OsStr>>(path: P) -> Result<()> {
    Command::new(path)
        .arg("toolchain")
        .arg("install")
        .arg("stable")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| "failed to install stable toolchain via rustup")?;

    Ok(())
}

fn install_rustup() -> Result<()> {
    let mut rustup_path = env::temp_dir();
    rustup_path.push("rustup.sh");

    let rustup_script = util::http_download_file("https://sh.rustup.rs")?;

    fs::write(rustup_path.clone(), rustup_script)?;

    // TODO: use libc here
    Command::new("chmod")
        .arg("+x")
        .arg(rustup_path.clone())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    Command::new("sh")
        .arg("-c")
        .arg(rustup_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    Ok(())
}

fn install_modules() -> Result<()> {
    println!("Running in interactive mode. You can always re-run memflowup to install additional packages, or to different paths.");

    let system_wide = util::user_input_boolean(
        "do you want to install the initial packages system-wide?",
        true,
    )?;

    update_index(system_wide)?;

    let packages = load_packages(system_wide)?;

    let branch = util::user_input(
        "which channel do you want to use?",
        &["stable", "development"],
        0,
    )
    .map(|r| r != 0)
    .map(<_>::into)?;

    let db = load_database(branch, system_wide)?;

    println!("using {} channel", branch.filename());

    println!();

    println!("Available packages:");

    for (i, package) in packages
        .iter()
        .filter(|p| p.is_in_channel(branch))
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

    println!();

    println!("Type packages to install by number, name, or type * for all:");

    let mut output = String::new();
    std::io::stdin().read_line(&mut output).unwrap();

    let trimmed = output.trim();

    let install_all = trimmed == "*";

    let (indices, names): (Vec<_>, Vec<_>) = trimmed
        .split_whitespace()
        .flat_map(|s| s.split(','))
        .flat_map(|s| s.split(';'))
        .partition(|s| s.chars().all(|c| c.is_numeric()));

    let indices = indices
        .into_iter()
        .map(str::parse::<usize>)
        .map(std::result::Result::unwrap)
        .collect::<Vec<_>>();

    for (_, p) in packages
        .into_iter()
        .filter(|p| p.is_in_channel(branch))
        .filter(Package::supported_by_platform)
        .enumerate()
        .filter(|(i, p)| install_all || indices.contains(i) || names.contains(&p.name.as_str()))
    {
        println!("Installing {}", p.name);
        p.install_source(branch, &PackageOpts::system_wide(system_wide))?;
    }

    println!("Initial setup done!");

    Ok(())
}
