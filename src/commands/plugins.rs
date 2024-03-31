//! Clap subcommand to list all installed plugins

use std::collections::HashSet;

use clap::{Arg, ArgAction, ArgMatches, Command};
use memflow_registry_client::shared::PluginVariant;

use crate::{
    error::Result,
    util::{self, LocalPlugin},
};

#[inline]
pub fn metadata() -> Command {
    Command::new("plugins")
        .subcommand_required(true)
        .subcommands([
            Command::new("list")
                .alias("ls")
                .args([Arg::new("plugin_name")
                    .help("name of the plugin as an additional filter")
                    .action(ArgAction::Set)]),
            Command::new("clean").alias("purge"),
            Command::new("remove")
                .alias("rm")
                .args([Arg::new("plugin_uri")
                    .help("uri of the plugin in the form of [registry]/[name]:[version]")
                    .action(ArgAction::Append)]),
        ])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", matches)) => {
            super::print_plugin_versions_header();
            list_local_plugins(matches.get_one::<String>("plugin_name").map(String::as_str)).await
        }
        Some(("remove", matches)) => {
            let plugin_uris = matches
                .get_many::<String>("plugin_uri")
                .unwrap_or_default()
                .cloned()
                .collect::<Vec<_>>();

            for plugin_uri in plugin_uris.iter() {
                remove_local_plugin_by_uri(plugin_uri).await?;
            }

            Ok(())
        }
        Some(("clean", _)) => {
            let orphaned = remove_orphaned_plugins().await?;
            let old_versions = remove_old_plugin_versions().await?;
            println!(
                "{} Plugins cleaned, removed {} plugins.",
                console::style("[=]").bold().dim().green(),
                orphaned + old_versions,
            );
            Ok(())
        }
        _ => unreachable!(),
    }
}

async fn list_local_plugins(plugin_name: Option<&str>) -> Result<()> {
    let plugins = util::local_plugins().await?;
    for plugin in plugins.into_iter() {
        // optionally filter by plugin name
        if let Some(plugin_name) = plugin_name {
            if plugin.variant.descriptor.name != plugin_name {
                continue;
            }
        }

        println!(
            "{0: <16} {1: <16} {2: <12} {3: <4} {4: <8} {5: <65} {6:}",
            plugin.variant.descriptor.name,
            plugin.variant.descriptor.version,
            format!(
                "{:?}/{:?}",
                plugin.variant.descriptor.file_type, plugin.variant.descriptor.architecture
            )
            .to_ascii_lowercase(),
            plugin.variant.descriptor.plugin_version,
            &plugin.variant.digest[..7],
            plugin.variant.digest,
            plugin.variant.created_at,
        );
    }
    Ok(())
}

async fn remove_local_plugin_by_uri(plugin_uri_str: &str) -> Result<()> {
    match util::find_local_plugin(plugin_uri_str).await {
        Ok(plugin) => remove_local_plugin(&plugin).await,
        Err(err) => {
            println!(
                "{} Plugin `{}` not found",
                console::style("[X]").bold().dim().red(),
                plugin_uri_str
            );
            Err(err)
        }
    }
}

async fn remove_local_plugin(local_plugin: &LocalPlugin) -> Result<()> {
    // delete plugin binary
    if let Err(err) = tokio::fs::remove_file(&local_plugin.plugin_file_name).await {
        println!(
            "{} Unable to delete plugin {:?}: {}",
            console::style("[X]").bold().dim().red(),
            local_plugin
                .plugin_file_name
                .file_name()
                .unwrap_or_default()
                .to_os_string(),
            err
        );
        return Err(err.into());
    }

    // delete meta file
    if let Err(err) = tokio::fs::remove_file(&local_plugin.meta_file_name).await {
        println!(
            "{} Unable to delete .meta file for plugin {:?}: {}",
            console::style("[X]").bold().dim().red(),
            local_plugin
                .meta_file_name
                .file_name()
                .unwrap_or_default()
                .to_os_string(),
            err
        );
        return Err(err.into());
    }

    println!(
        "{} Deleted plugin: {:?}",
        console::style("[=]").bold().dim().green(),
        local_plugin.plugin_file_name.as_os_str(),
    );

    Ok(())
}

/// Removes all plugins which do not have a proper .meta file associated with them.
async fn remove_orphaned_plugins() -> Result<usize> {
    let mut orphaned_plugins = 0;

    let paths = std::fs::read_dir(util::plugins_path())?;
    for path in paths.filter_map(|p| p.ok()) {
        if let Some(extension) = path.path().extension() {
            // TODO: should we only check for plugin_extension here?
            if extension.to_str().unwrap_or_default() == util::plugin_extension() {
                // check if the corresponding .meta file exists
                let mut meta_file_name = path.path();
                meta_file_name.set_extension("meta");

                let orphaned = if meta_file_name.exists() {
                    if let Ok(metadata) = serde_json::from_str::<PluginVariant>(
                        &tokio::fs::read_to_string(meta_file_name).await?,
                    ) {
                        let bytes = tokio::fs::read(path.path()).await?;
                        let digest = sha256::digest(&bytes[..]);
                        if metadata.digest == digest {
                            None
                        } else {
                            // digest in .meta is not matching file on disk
                            Some("checksum mismatch in .meta file")
                        }
                    } else {
                        // invalid .meta file
                        Some("corrupted .meta file")
                    }
                } else {
                    // .meta file does not exist
                    Some(".meta file missing")
                };

                if let Some(reason) = orphaned {
                    // TODO: try parse metafile and check digest to be triple sure

                    // remove plugin
                    if let Err(err) = tokio::fs::remove_file(path.path()).await {
                        println!(
                            "{} Unable to delete plugin {:?}: {}",
                            console::style("[X]").bold().dim().red(),
                            path.path().file_name().unwrap_or_default().to_os_string(),
                            err
                        );
                        return Err(err.into());
                    }

                    // try to remove meta file (this is allowed to fail)
                    let mut meta_file_name = path.path();
                    meta_file_name.set_extension("meta");
                    if meta_file_name.exists() {
                        // only try to delete the file if it exists, so we do not print an error in all cases
                        if let Err(err) = tokio::fs::remove_file(meta_file_name).await {
                            println!(
                                "{} Unable to delete .meta file for plugin {:?}: {}",
                                console::style("[X]").bold().dim().red(),
                                path.path().file_name().unwrap_or_default().to_os_string(),
                                err
                            );
                        }
                    }

                    println!(
                        "{} Deleted orphaned plugin: {:?} ({})",
                        console::style("[=]").bold().dim().green(),
                        path.path().as_os_str(),
                        reason
                    );

                    orphaned_plugins += 1;
                    continue;
                }
            }
        }
    }

    Ok(orphaned_plugins)
}

/// Removes all plugins which do not have a proper .meta file associated with them.
async fn remove_old_plugin_versions() -> Result<usize> {
    let mut old_plugin_versions = 0;

    // get a list of pre-sorted plugins, we simply need to delete all but the first occurence of each plugin
    let mut seen = HashSet::new();
    let plugins = util::local_plugins().await?;
    for plugin in plugins.iter() {
        if seen.contains(&plugin.variant.descriptor.name) {
            // delete the file if we have seen a newer version already
            remove_local_plugin(plugin).await?;
            old_plugin_versions += 1;
        } else {
            // add the file to our "seen" list
            seen.insert(plugin.variant.descriptor.name.clone());
        }
    }

    Ok(old_plugin_versions)
}
