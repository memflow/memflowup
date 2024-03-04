use crate::database::Branch;
use crate::package;
use crate::util;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::process::{Command, Stdio};

use inquire::Confirm;
use inquire::MultiSelect;
use inquire::Select;
use log::{info, warn};

use crate::package::*;

use crate::Result;

pub fn setup_mode(load_opts: PackageLoadOpts) -> Result<()> {
    let binary_install = {
        let ans = Confirm::new("Do you want to install binary packages?")
            .with_default(true)
            .with_help_message(
                "Some components require additional third-party libraries to be built from source.",
            )
            .prompt();

        matches!(ans, Ok(true) | Err(_))
    };
    let source_install = !binary_install;

    if source_install {
        // 1. ensure rustup / cargo is installed in PATH
        ensure_rust()?;
    }

    // 2. install memflowup in PATH

    // 3. install default set of connectors for the current platform
    install_modules(load_opts, source_install)
}

fn ensure_rust() -> Result<()> {
    match which::which("cargo") {
        Ok(cargo_dir) => {
            info!("cargo found at {:?}", cargo_dir);
            // TODO: check rust version
            Ok(())
        }
        Err(_) => {
            warn!("cargo not found");
            if !cfg!(windows) {
                let install_rustup = {
                    let ans = Confirm::new("Do you want to install rust via rustup now?")
                        .with_default(true)
                        .with_help_message(
                            "Some components require additional third-party libraries to be built from source.",
                        )
                        .prompt();

                    matches!(ans, Ok(true) | Err(_))
                };

                if install_rustup {
                    info!("cargo not found, installing via rustup");
                    install_rust()
                } else {
                    println!("rust/cargo not found. please install it manually.");
                    Err("rust/cargo not found. please install it manually.".into())
                }
            } else {
                println!("rust/cargo not found. please install it manually.");
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

fn install_modules(load_opts: PackageLoadOpts, from_source: bool) -> Result<()> {
    println!("Running in interactive mode. You can always re-run memflowup to install additional packages, or to different paths.");

    let user_only = {
        let ans = Confirm::new("Do you want to install packages for the current-user only?")
            .with_default(true)
            .with_help_message(
                "Installing packages for the current user will place them in ~/.local/lib/memflow/",
            )
            .prompt();

        matches!(ans, Ok(true) | Err(_))
    };
    let system_wide = !user_only;

    if !load_opts.ignore_upstream {
        update_index(system_wide)?;
    }

    let packages = load_packages(system_wide, load_opts)?;

    let branch = {
        let options = vec!["stable", "experimental"];
        let ans = Select::new("Which channel do you want to use?", options).prompt();

        match ans {
            Ok("stable") | Err(_) => Branch::Stable,
            Ok("experimental") => Branch::Dev,
            _ => Branch::Stable,
        }
    };

    package::list(system_wide, branch, from_source, load_opts)?;

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
        .filter(|p| p.supports_install_mode(branch, from_source))
        .filter(Package::supported_by_platform)
        .enumerate()
        .filter(|(i, p)| install_all || indices.contains(i) || names.contains(&p.name.as_str()))
    {
        println!("Installing {}", p.name);
        p.install(branch, &PackageOpts::base_opts(system_wide, from_source))?;
    }

    // since we install over cargo we skip adding memflowup into path for now.
    /*
    let memflowup_path = env::args()
        .next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
        .expect("Unable to get the path of the memflowup application");

    if util::user_input_boolean("Do you want to install memflowup in your system?", true)? {
        util::copy_file(
            &memflowup_path,
            &util::executable_dir(true).join("memflowup"),
            true,
        )?;
    }
    */

    println!();

    println!("Initial setup done!");

    Ok(())
}
