use crate::scripting;
use crate::util;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::process::{Command, Stdio};

use log::{info, warn};

/// TESTING
/// TESTING
/// TESTING

// TODO: result

/// TESTING
/// TESTING
/// TESTING

pub fn setup_mode() {
    // 1. ensure rustup / cargo is installed in PATH
    ensure_rust();

    // 2. ask the user what packages he wants to install (filtered by the current OS)
    // 2.1. ask user if he wants nightly or stable versions
    // 2.2 ask the user wether to put them into global / local directory -> ask for sudo permissions on linux
    scripting::execute("memflow-coredump.rhai");

    // 4. done :)
}

fn ensure_rust() {
    match which::which("cargo") {
        Ok(cargo_dir) => {
            info!("cargo found at {:?}", cargo_dir);
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
            info!("rustup found at {:?}", rustup_path);
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
