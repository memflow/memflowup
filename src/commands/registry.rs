//! Clap subcommand to query the registry

use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};

use crate::error::Result;

use super::config::read_config;

#[inline]
pub fn metadata() -> Command {
    Command::new("registry")
        .subcommand_required(false)
        .subcommands([
            Command::new("list").alias("ls").args([
                Arg::new("plugin_name")
                    .help("name of the plugin as an additional filter")
                    .action(ArgAction::Set),
                Arg::new("versions")
                    .short('l')
                    .long("versions")
                    .help("show the long listing, with each version of each plugin on its own line")
                    .action(ArgAction::SetTrue),
                Arg::new("all-archs")
                    .short('a')
                    .long("all-archs")
                    .alias("all-architectures")
                    .help("shows plugins regardless of the current architecture")
                    .action(ArgAction::SetTrue),
                Arg::new("limit")
                    .long("limit")
                    .value_parser(value_parser!(usize))
                    .default_value("25")
                    .help("the amount of plugins to show in the listing")
                    .action(ArgAction::Set),
            ]),
            Command::new("remove").alias("rm").args([
                Arg::new("plugin_digest")
                    .required(true)
                    .help("full or short digest of the plugin")
                    .action(ArgAction::Set),
                Arg::new("token")
                    .short('t')
                    .long("token")
                    .help("bearer token used in the upload request")
                    .action(ArgAction::Set),
            ]),
        ])
        .args([Arg::new("registry")
            .short('r')
            .long("registry")
            .help("custom registry to use")
            .action(ArgAction::Set)])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let config = read_config().await?;
    let registry = matches
        .get_one::<String>("registry")
        .map(String::as_str)
        .or(config.registry.as_deref());

    // TODO: list local + remote plugins
    // TODO: allow changing to another registry provider
    match matches.subcommand() {
        Some(("list", matches)) => {
            let all_archs = matches.get_flag("all-archs");

            if let Some(plugin_name) = matches.get_one::<String>("plugin_name") {
                let limit = matches.get_one::<usize>("limit").unwrap();
                print_plugin_versions_header();
                list_plugin_versions(registry, plugin_name, all_archs, *limit).await?;
            } else {
                let versions = matches.get_flag("versions");

                // list all plugins
                let plugins = memflow_registry::client::plugins(registry).await?;
                if versions {
                    // TODO: display plugins that do not have a version for our current os?
                    print_plugin_versions_header();
                    for plugin in plugins.iter() {
                        list_plugin_versions(registry, &plugin.name, all_archs, 1).await?;
                    }
                } else {
                    println!("{0: <16} DESCRIPTION", "NAME");
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
            let token = matches.get_one::<String>("token").or(config.token.as_ref());

            if let Err(err) =
                memflow_registry::client::delete(registry, token.map(String::as_str), plugin_digest)
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

#[allow(clippy::print_literal)]
#[inline]
fn print_plugin_versions_header() {
    println!(
        "{0: <16} {1: <16} {2: <12} {3: <4} {4: <8} {5: <65} {6:}",
        "NAME", "VERSION", "ARCH", "ABI", "DIGEST", "DIGEST_LONG", "CREATED"
    );
}

async fn list_plugin_versions(
    registry: Option<&str>,
    plugin_name: &str,
    all_archs: bool,
    limit: usize,
) -> Result<()> {
    // list versions of a specific plugin
    let plugins =
        memflow_registry::client::plugin_versions(registry, plugin_name, all_archs, None, limit)
            .await?;
    // TODO: dedup versions

    for variant in plugins.iter() {
        println!(
            "{0: <16} {1: <16} {2: <12} {3: <4} {4: <8} {5: <65} {6:}",
            plugin_name,
            variant.descriptor.version,
            format!(
                "{:?}/{:?}",
                variant.descriptor.file_type, variant.descriptor.architecture
            )
            .to_ascii_lowercase(),
            variant.descriptor.plugin_version,
            &variant.digest[..7],
            variant.digest,
            variant.created_at,
        );
    }

    Ok(())
}
