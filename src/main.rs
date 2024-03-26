mod commands;
mod ensure_rust;
mod error;
mod github_api;
mod util;

use std::{process::exit, time::Duration};

use clap::*;
use crates_io_api::SyncClient;
use inquire::Confirm;

use error::{Error, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = parse_args();

    // check if we run as root
    check_root()?;

    // check for update after we parsed the args
    if !matches.get_flag("skip-version-check") {
        #[cfg(not(debug_assertions))]
        check_for_update().ok();
        #[cfg(debug_assertions)]
        println!("Skipping update check in debug mode.");
    }

    // set log level
    env_logger::init();

    match matches.subcommand() {
        Some(("push", matches)) => commands::push::handle(matches).await,
        Some(("pull", matches)) => commands::pull::handle(matches).await,
        Some(("registry", matches)) => commands::registry::handle(matches).await,
        Some(("plugins", matches)) => commands::plugins::handle(matches).await,
        Some(("build", matches)) => commands::build::handle(matches).await,
        Some(("config", matches)) => commands::config::handle(matches).await,
        _ => Ok(()),
    }
}

fn parse_args() -> ArgMatches {
    Command::new("memflowup")
        .arg_required_else_help(true)
        .subcommand_required(true)
        .version(crate_version!())
        .author(crate_authors!())
        .arg(
            Arg::new("skip-version-check")
                .long("skip-version-check")
                .action(ArgAction::SetTrue),
        )
        .subcommands([
            commands::build::metadata(),
            commands::config::metadata(),
            commands::plugins::metadata(),
            commands::pull::metadata(),
            commands::push::metadata(),
            commands::registry::metadata(),
        ])
        .get_matches()
}

#[allow(unused)]
fn check_for_update() -> Result<()> {
    let client = SyncClient::new("memflowup", Duration::from_millis(1000))
        .map_err(|err| Error::Http(err.to_string()))?;
    let memflowup = client.get_crate(crate_name!())?;

    // find latest non-yanked version
    if let Some(latest_version) = memflowup.versions.iter().find(|v| !v.yanked) {
        if latest_version.num != crate_version!() {
            println!("An update for memflowup is available.");
            println!();
            println!("To install the new version run:");
            println!("$ cargo install memflowup --force");
            println!();
            println!("More information about installing memflowup can be found at https://memflow.io/quick_start/");

            let ans = Confirm::new("Do you want to continue without updating?")
                .with_default(false)
                .with_help_message(
                    "Some features might not work properly with an outdated version.",
                )
                .prompt();

            match ans {
                Ok(false) | Err(_) => exit(0),
                _ => (),
            }
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
fn check_root() -> Result<()> {
    let is_root = unsafe { libc::getuid() } == 0;
    if is_root {
        println!("memflowup has been started as the root user or via sudo.");
        println!();
        println!("By default everything should be installed under your local user home directory and not the home directory of the root user.");
        println!("If you want to continue installing components as the root user they will be placed in /root/.local/lib/memflow instead of $HOME/.local/lib/memflow.");
        println!("This might cause issues in case you do not run your memflow program via root/sudo as well.");

        let ans = Confirm::new("Do you want to continue running memflowup as root?")
            .with_default(false)
            .with_help_message("Some things might not work as intended.")
            .prompt();

        match ans {
            Ok(false) | Err(_) => exit(0),
            _ => (),
        }
    }
    Ok(())
}

#[cfg(not(target_family = "unix"))]
fn check_root() -> Result<()> {
    Ok(())
}
