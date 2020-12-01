use crate::util;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use log::{info, warn};

pub fn setup_mode() {
    // 1. ensure rustup / cargo is installed in PATH
    ensure_rust();

    // 2. ask the user what packages he wants to install (filtered by the current OS)
    install_connector_branch("memflow-coredump", "master");

    // 2.1. ask user if he wants nightly or stable versions

    // 3. install connectors (with connector specific quirks / additional dependencies)
    // 3.1 download the corresponding git repository in a temp folder, at best at the latest tagged version
    // 3.2 compile the connector and install it
    // 3.3 ask the user wether to put them into global / local directory -> ask for sudo permissions on linux

    // 4. done :)
}

fn ensure_rust() {
    match which::which("cargo") {
        Ok(cargo_dir) => {
            info!("cargo found at '{:?}'", cargo_dir);
        }
        Err(_) => {
            warn!("cargo not found, trying to install via rustup");
            install_rust();
        }
    }
}

// TODO: windows ->
/// Downloads and executes rustup or panics
fn install_rust() {
    match which::which("rustup") {
        Ok(rustup_path) => {
            info!("rustup found at '{:?}'", rustup_path.clone());
            install_rust_toolchain(rustup_path);
        }
        Err(_) => {
            warn!("rustup is not installed, trying to download");
            install_rustup();
        }
    }
}

fn install_rust_toolchain<P: AsRef<OsStr>>(path: P) {
    // TODO: ask user
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
        util::download_file("https://sh.rustup.rs").expect("unable to download rustup script");
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

/// Installs the given connector from git
fn install_connector_branch(connector_name: &str, connector_branch: &str) {
    // 1. we need to query all available branches
    // https://api.github.com/repos/memflow/memflow-coredump/branches

    // if we want to install a stable version of the connector
    // we should pick the latest tag or master branch via this api:
    // https://api.github.com/repos/memflow/memflow-coredump/tags
    // this will also give us a zip file in the zipball_url / tarball_url

    // if we want to install a specific branch just download the branch
    // specific branches can then be downloaded as follows:
    // https://github.com/memflow/memflow-coredump/archive/master.tar.gz

    info!("downloading connector: {}", connector_name);
    let mut connector_path = env::temp_dir();
    connector_path.push(format!("{}_{}.tar.gz", connector_name, connector_branch));
    let connector_zip = util::download_file(&format!(
        "https://api.github.com/repos/memflow/{}/tarball/{}",
        connector_name, connector_branch
    ))
    .expect("unable to download connector archive");
    fs::write(connector_path.clone(), connector_zip)
        .expect("unable to write rustup script to temp directory");

    // 2. unzip the archive in the temp directory
    // TODO: use crates.io zip = "0.5"
    Command::new("tar")
        .arg("-xvf")
        .arg(connector_path.clone())
        .arg("--strip")
        .arg("1")
        .arg("--directory")
        .arg(format!("{}-{}", connector_name, connector_branch))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to extract connector tarball");

    // 3. run `cargo build --release --all-features` in the folder
    let mut connector_out_path = env::temp_dir();
    connector_out_path.push(format!("{}-{}", connector_name, connector_branch));
    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--all-features")
        .current_dir(connector_out_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to compile connector");
}
