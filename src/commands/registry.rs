//! Clap subcommand to query the registry

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::error::Result;

use super::config::read_config;

#[inline]
pub fn metadata() -> Command {
    Command::new("registry")
        .subcommand_required(false)
        .subcommands([
            Command::new("list").alias("ls").args([
                Arg::new("plugin_name").action(ArgAction::Set),
                Arg::new("versions")
                    .short('l')
                    .long("versions")
                    .help("show the long listing, with each version of each plugin on its own line")
                    .action(ArgAction::SetTrue),
            ]),
            Command::new("remove").alias("rm").args([
                Arg::new("plugin_digest")
                    .required(true)
                    .action(ArgAction::Set),
                Arg::new("token")
                    .short('t')
                    .long("token")
                    .help("the bearer token used in the upload request")
                    .action(ArgAction::Set),
            ]),
        ])
        .args([Arg::new("registry")
            .short('r')
            .long("registry")
            .help("pushes the plugin to a custom registry")
            .action(ArgAction::Set)])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let config = read_config().await?;
    let registry = matches
        .get_one::<String>("registry")
        .map(String::as_str)
        .or_else(|| config.registry.as_deref());

    // TODO: list local + remote plugins
    // TODO: allow changing to another registry provider
    match matches.subcommand() {
        Some(("list", matches)) => {
            if let Some(plugin_name) = matches.get_one::<String>("plugin_name") {
                print_plugin_versions_header();
                list_plugin_versions(registry, plugin_name, 50).await?;
            } else {
                let versions = matches.get_flag("versions");

                // list all plugins
                let plugins = memflow_registry_client::plugins(registry).await?;
                if versions {
                    // TODO: display plugins that do not have a version for our current os?
                    print_plugin_versions_header();
                    for plugin in plugins.iter() {
                        list_plugin_versions(registry, &plugin.name, 1).await?;
                    }
                } else {
                    println!("{0: <16} {1}", "NAME", "DESCRIPTION");
                    for plugin in plugins.iter() {
                        println!("{0: <16} {1}", plugin.name, plugin.description);
                    }
                }
            }

            Ok(())
        }
        Some(("remove", matches)) => {
            let config = read_config().await?;
            let plugin_digest = matches.get_one::<String>("plugin_digest").unwrap();
            let token = matches
                .get_one::<String>("token")
                .or_else(|| config.token.as_ref());

            if let Err(err) =
                memflow_registry_client::delete(registry, token.map(String::as_str), plugin_digest)
                    .await
            {
                println!(
                    "{} Unable to delete plugin entry from registry: {}",
                    console::style("[X]").bold().dim().red(),
                    err
                );
            }

            Ok(())
        }
        _ => {
            unreachable!()
        }
    }
}

fn print_plugin_versions_header() {
    println!(
        "{0: <16} {1: <16} {2: <16} {3: <16} {4: <64} {5}",
        "NAME", "VERSION", "PLUGIN_VERSION", "DIGEST", "DIGEST_LONG", "CREATED"
    );
}
async fn list_plugin_versions(
    registry: Option<&str>,
    plugin_name: &str,
    limit: usize,
) -> Result<()> {
    // list versions of a specific plugin
    let plugins =
        memflow_registry_client::plugin_versions(registry, plugin_name, None, limit).await?;
    // TODO: dedup versions

    for plugin in plugins.iter() {
        println!(
            "{0: <16} {1: <16} {2: <16} {3: <16} {4: <64} {5}",
            plugin_name,
            plugin.descriptor.version,
            plugin.descriptor.plugin_version,
            &plugin.digest[..7],
            plugin.digest,
            plugin.created_at.to_string(),
        );
    }

    Ok(())
}
