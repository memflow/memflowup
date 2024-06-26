//! Clap subcommand to pull plugins from a registry

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    error::{Error, Result},
    util,
};
use memflow_registry::{
    PluginUri, SignatureVerifier, MEMFLOW_DEFAULT_REGISTRY, MEMFLOW_DEFAULT_REGISTRY_VERIFYING_KEY,
};

use super::config::read_config;

#[inline]
pub fn metadata() -> Command {
    Command::new("pull").args([
        Arg::new("plugin_uri").action(ArgAction::Append),
        Arg::new("all")
            .short('a')
            .long("all")
            .help("pulls the latest version of all available plugins")
            .action(ArgAction::SetTrue),
        Arg::new("force")
            .short('f')
            .long("force")
            .help("forces download of the plugin even if it is already installed")
            .action(ArgAction::SetTrue),
        Arg::new("registry")
            .short('r')
            .long("registry")
            .help("pulls the plugin from a custom registry")
            .action(ArgAction::Set),
        Arg::new("pub-key")
            .short('p')
            .long("pub-key")
            .help("public key used to verify the binary signature (this is required for self-hosted registries)")
            .action(ArgAction::Set),
        ])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let config = read_config().await?;
    let plugin_uris = matches
        .get_many::<String>("plugin_uri")
        .unwrap_or_default()
        .cloned()
        .collect::<Vec<_>>();
    let all = matches.get_flag("all");
    let force = matches.get_flag("force");
    let registry = matches
        .get_one::<String>("registry")
        .map(String::as_str)
        .or(config.registry.as_deref());
    let pub_key_file = matches
        .get_one::<String>("pub-key")
        .map(Path::new)
        .or(config.pub_key_file.as_deref());

    // TODO: support custom registry for wildcard
    if all {
        let plugins = memflow_registry::client::plugins(None).await?;
        for plugin in plugins.iter() {
            if let Err(err) = pull(registry, &plugin.name, force, pub_key_file).await {
                println!(
                    "{} Error downloading plugin {:?}: {}",
                    console::style("[X]").bold().dim().red(),
                    plugin.name,
                    err
                );
            }
        }
    } else {
        // TODO: parallel downloads
        for plugin_uri in plugin_uris.iter() {
            if let Err(err) = pull(registry, plugin_uri, force, pub_key_file).await {
                println!(
                    "{} Error downloading plugin {:?}: {}",
                    console::style("[X]").bold().dim().red(),
                    plugin_uri,
                    err
                );
            }
        }
    }

    Ok(())
}

async fn pull(
    registry: Option<&str>,
    plugin_uri: &str,
    force: bool,
    pub_key: Option<&Path>,
) -> Result<()> {
    // load the signature verifier
    let verifier = if let Some(pub_key) = pub_key {
        // load custom public key
        SignatureVerifier::new(pub_key)
    } else {
        // use default bundled public key
        SignatureVerifier::with_str(MEMFLOW_DEFAULT_REGISTRY_VERIFYING_KEY)
    }?;

    // find the correct plugin variant based on the input arguments
    let plugin_uri = PluginUri::with_defaults(
        plugin_uri,
        registry.unwrap_or(MEMFLOW_DEFAULT_REGISTRY),
        "latest",
    )?;
    let variant = memflow_registry::client::find_by_uri(&plugin_uri, false, None).await?;

    // query file metadata for variant
    let metadata = memflow_registry::client::metadata(&plugin_uri, &variant).await?;

    // check if file already exists
    let file_name = util::plugin_file_name(&metadata);
    if !force && file_name.exists() {
        let bytes = tokio::fs::read(&file_name).await?;
        let digest = sha256::digest(&bytes[..]);

        // check if the plugin digest matches with the one from memflow-registry
        if variant.digest == digest {
            println!(
                "{} Plugin {:?} already exists with the same checksum, skipping download.",
                console::style("[-]").bold().dim().yellow(),
                file_name.file_name().unwrap()
            );
            return Ok(());
        } else {
            println!(
                "{} Plugin {:?} already exists with a different checksum, redownloading.",
                console::style("[-]").bold().dim().yellow(),
                file_name.file_name().unwrap()
            );
        }
    }

    // query file and download to memory
    let response = memflow_registry::client::download(&plugin_uri, &variant).await?;
    let buffer = util::read_response_with_progress(response).await?;

    // verify file signature
    if verifier
        .is_valid(buffer.as_ref(), &variant.signature)
        .is_err()
    {
        println!(
            "{} Plugin signature verification failed (in case you're using a self-hosted registry, please provide a custom public key)",
            console::style("[X]").bold().dim().red(),
        );
        return Err(Error::Signature("plugin verification failed".to_owned()));
    }

    // write file (signature matches)
    let mut file = File::create(&file_name).await?;
    file.write_all(buffer.as_ref()).await?;
    file.flush().await?;

    println!(
        "{} Wrote plugin to: {:?}",
        console::style("[=]").bold().dim().green(),
        file_name.as_os_str(),
    );

    // store .meta file of plugin containing all relevant information
    // TODO: this does not contain all plugins in this file - allow querying that from memflow-registry as well
    let mut file_name = file_name.clone();
    file_name.set_extension("meta");
    tokio::fs::write(&file_name, serde_json::to_string_pretty(&metadata)?).await?;

    println!(
        "{} Wrote plugin metadata to: {:?}",
        console::style("[=]").bold().dim().green(),
        file_name.as_os_str(),
    );

    Ok(())
}
