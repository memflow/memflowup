//! Clap subcommand to push plugins in a registry

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use memflow_registry_client::shared::SignatureGenerator;

use crate::error::{Error, Result};

use super::config::read_config;

#[inline]
pub fn metadata() -> Command {
    Command::new("push").args([
        Arg::new("file_name")
            .help("file to upload")
            .required(true)
            .action(ArgAction::Set),
        Arg::new("registry")
            .short('r')
            .long("registry")
            .help("pushes the plugin to a custom registry")
            .action(ArgAction::Set),
        Arg::new("token")
            .short('t')
            .long("token")
            .help("bearer token used in the upload request")
            .action(ArgAction::Set),
        Arg::new("priv-key")
            .short('p')
            .long("priv-key")
            .help("private key used to sign the binary")
            .action(ArgAction::Set),
    ])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let config = read_config().await?;
    let file_name = matches.get_one::<String>("file_name").unwrap();
    let registry = matches
        .get_one::<String>("registry")
        .map(String::as_str)
        .or(config.registry.as_deref());
    let token = matches.get_one::<String>("token").or(config.token.as_ref());
    let priv_key_file = matches
        .get_one::<String>("priv-key")
        .map(Path::new)
        .or(config.priv_key_file.as_deref());
    let priv_key_file = match priv_key_file {
        Some(v) => v,
        None => {
            println!(
                "{} Private key file is required for signing. Either configure it via `memflowup config` or the `--priv-key` argument",
                console::style("[X]").bold().dim().red(),
            );
            return Err(Error::NotFound("private key file not found".to_owned()));
        }
    };

    // TODO: upload progress

    let mut generator = SignatureGenerator::new(priv_key_file)?;
    match memflow_registry_client::upload(
        registry,
        token.map(String::as_str),
        file_name,
        &mut generator,
    )
    .await
    {
        Ok(_) => {
            println!(
                "{} Uploaded plugin {:?}",
                console::style("[=]").bold().dim().green(),
                file_name
            );
        }
        Err(msg) => {
            println!(
                "{} Unable to upload plugin {:?}: {}",
                console::style("[X]").bold().dim().red(),
                file_name,
                msg
            );
        }
    }

    Ok(())
}
