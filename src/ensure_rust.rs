use std::{
    ffi::OsStr,
    process::{Command, Stdio},
};

use inquire::Confirm;

use crate::error::Result;

/// Checks if cargo / rust installed properly or installs it
pub async fn ensure_rust() -> Result<()> {
    match which::which("cargo") {
        Ok(cargo_dir) => {
            println!("cargo found at {:?}", cargo_dir);
            // TODO: check rust version
            Ok(())
        }
        Err(_) => {
            println!("cargo not found");
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
                    log::info!("cargo not found, installing via rustup");
                    install_rust().await
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
async fn install_rust() -> Result<()> {
    match which::which("rustup") {
        Ok(rustup_path) => {
            println!("rustup found at {:?}", rustup_path);
            install_rust_toolchain(rustup_path)
        }
        Err(_) if !cfg!(unix) => {
            println!("rustup is not installed, trying to download");
            install_rustup().await.and_then(|_| {
                install_rust_toolchain(
                    which::which("rustup").expect("No rustup found after installing rustup!"),
                )
            })
        }
        _ => {
            println!("rustup is not installed, setup manually!");
            Err("Please install rustup".into())
        }
    }
}

// TODO: windows / mac support
fn install_rust_toolchain<P: AsRef<OsStr>>(path: P) -> Result<()> {
    std::process::Command::new(path)
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

async fn install_rustup() -> Result<()> {
    let mut rustup_path = std::env::temp_dir();
    rustup_path.push("rustup.sh");

    let response = reqwest::get("https://sh.rustup.rs").await?;
    tokio::fs::write(rustup_path.clone(), response.text().await?).await?;

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
