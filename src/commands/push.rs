//! Clap subcommand to push plugins in a registry

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use memflow_registry_client::shared::SignatureGenerator;

use crate::{
    error::{Error, Result},
    util,
};

use super::config::read_config;

// either plugin_uri or file is set
#[inline]
pub fn metadata() -> Command {
    Command::new("push").args([
        Arg::new("plugin_uris_or_files")
            .help("list of plugin uris or filenames")
            .required(true)
            .action(ArgAction::Append),
        Arg::new("file")
            .short('f')
            .long("file")
            .help("upload a plugin binary directly")
            .action(ArgAction::SetTrue),
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
    let plugin_uris_or_files = matches
        .get_many::<String>("plugin_uris_or_files")
        .unwrap_or_default()
        .cloned()
        .collect::<Vec<_>>();
    let file = matches.get_flag("file");
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

    if !file {
        // try to find the plugin first, then upload it to the registry
        for plugin_uri in plugin_uris_or_files.iter() {
            match util::find_local_plugin(plugin_uri).await {
                Ok((plugin_file_name, _)) => {
                    upload_plugin_file(
                        registry,
                        token.map(String::as_str),
                        priv_key_file,
                        &plugin_file_name,
                    )
                    .await?;
                }
                Err(_) => {
                    println!(
                        "{} Plugin `{}` not found",
                        console::style("[X]").bold().dim().red(),
                        plugin_uri
                    );
                }
            }
        }
    } else {
        for file_name in plugin_uris_or_files.iter() {
            // upload a file directly
            upload_plugin_file(
                registry,
                token.map(String::as_str),
                priv_key_file,
                file_name,
            )
            .await?;
        }
    }

    Ok(())
}

async fn upload_plugin_file<P: AsRef<Path>>(
    registry: Option<&str>,
    token: Option<&str>,
    priv_key_file: &Path,
    file_name: P,
) -> Result<()> {
    // TODO: upload progress
    let mut generator = SignatureGenerator::new(priv_key_file)?;
    match memflow_registry_client::upload(registry, token, file_name.as_ref(), &mut generator).await
    {
        Ok(_) => {
            println!(
                "{} Uploaded plugin {:?}",
                console::style("[=]").bold().dim().green(),
                file_name.as_ref()
            );
        }
        Err(msg) => {
            println!(
                "{} Unable to upload plugin {:?}: {}",
                console::style("[X]").bold().dim().red(),
                file_name.as_ref(),
                msg
            );
        }
    }

    Ok(())
}
