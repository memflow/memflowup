use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::{error::Result, registry};

#[inline]
pub fn metadata() -> Command {
    Command::new("registry")
        .subcommand_required(false)
        .subcommands([Command::new("list").alias("ls").args([
            Arg::new("plugin_name").action(ArgAction::Set),
            Arg::new("versions")
                .short('l')
                .long("versions")
                .help("show the long listing, with each version of each plugin on its own line")
                .action(ArgAction::SetTrue),
        ])])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    // TODO: list local + remote plugins
    // TODO: allow changing to another registry provider
    match matches.subcommand() {
        Some(("list", matches)) => {
            if let Some(plugin_name) = matches.get_one::<String>("plugin_name") {
                print_plugin_versions_header();
                list_plugin_versions(plugin_name, 50).await?;
            } else {
                let versions = matches.get_flag("versions");

                // list all plugins
                let plugins = registry::plugins(None).await?;
                if versions {
                    // TODO: display plugins that do not have a version for our current os?
                    print_plugin_versions_header();
                    for plugin in plugins.iter() {
                        list_plugin_versions(&plugin.name, 1).await?;
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
        _ => {
            unreachable!()
        }
    }
}

fn print_plugin_versions_header() {
    println!(
        "{0: <16} {1: <16} {2: <16} {3}",
        "NAME", "VERSION", "DIGEST", "UPLOADED"
    );
}
async fn list_plugin_versions(plugin_name: &str, limit: usize) -> Result<()> {
    // list versions of a specific plugin
    let plugins = registry::plugin_versions(None, plugin_name, limit).await?;
    // TODO: dedup versions

    for plugin in plugins.iter() {
        println!(
            "{0: <16} {1: <16} {2: <16} {3}",
            plugin_name,
            plugin.descriptor.version,
            &plugin.digest[..7],
            plugin.created_at
        );
    }

    Ok(())
}
