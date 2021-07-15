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

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn install_paths() -> Vec<PathBuf> {
    vec![
        dirs::home_dir().unwrap().join(".local/lib/memflow"),
        "/usr/lib/memflow".into(),
    ]
}

#[cfg(target_os = "windows")]
fn install_paths() -> Vec<PathBuf> {
    vec![dirs::document_dir().unwrap().join("memflow")]
}

#[cfg(target_os = "macos")]
fn install_paths() -> Vec<PathBuf> {
    vec![
        dirs::home_dir().unwrap().join(".local/lib/memflow"),
        "/usr/lib/memflow".into(),
    ]
}

pub fn setup_mode() {
    // 1. ensure rustup / cargo is installed in PATH
    ensure_rust();

    // 2. install memflowup in PATH

    // 3. install default set of connectors for the current platform
    install_modules();
}

fn ensure_rust() {
    match which::which("cargo") {
        Ok(cargo_dir) => {
            info!("cargo found at {:?}", cargo_dir);
        }
        Err(_) => {
            warn!("cargo not found");
            if util::user_input_boolean("do you want memflowup to install rust via rustup?", true) {
                info!("cargo not found, installing via rustup");
                install_rust();
            } else {
                panic!("rust/cargo not found. please install it manually.");
            }
        }
    }
}

// TODO: windows / mac support
/// Downloads and executes rustup or panics
fn install_rust() {
    match which::which("rustup") {
        Ok(rustup_path) => {
            info!("rustup found at {:?}", rustup_path);
            install_rust_toolchain(rustup_path);
        }
        Err(_) => {
            warn!("rustup is not installed, trying to download");
            install_rustup();
        }
    }
}

// TODO: windows / mac support
fn install_rust_toolchain<P: AsRef<OsStr>>(path: P) {
    Command::new(path)
        .arg("toolchain")
        .arg("install")
        .arg("stable")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to install stable toolchain via rustup");
}

fn install_rustup() {
    let mut rustup_path = env::temp_dir();
    rustup_path.push("rustup.sh");
    let rustup_script =
        util::http_download_file("https://sh.rustup.rs").expect("unable to download rustup script");
    fs::write(rustup_path.clone(), rustup_script)
        .expect("unable to write rustup script to temp directory");

    // TODO: use libc here
    Command::new("chmod")
        .arg("+x")
        .arg(rustup_path.clone())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to set permission for rustup script");

    Command::new("sh")
        .arg("-c")
        .arg(rustup_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to execute rustup script");
}

fn install_modules() {
    let packages = load_packages();

    let dev_branch = true;

    println!(
        "using {} channel",
        if dev_branch { "dev" } else { "stable" }
    );

    println!();

    println!("Available packages:");

    for (i, package) in packages
        .iter()
        .filter(|p| p.is_in_channel(dev_branch))
        .enumerate()
    {
        println!("{}. {} - {:?}", i, package.name, package.ty);
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
        .map(Result::unwrap)
        .collect::<Vec<_>>();

    packages
        .into_iter()
        .filter(|p| p.is_in_channel(dev_branch))
        .enumerate()
        .filter(|(i, p)| install_all || indices.contains(i) || names.contains(&p.name.as_str()))
        .for_each(|(_, p)| {
            println!("Installing {}", p.name);
            p.install_source(dev_branch);
        });
}
