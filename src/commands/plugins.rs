//! Clap subcommand to list all installed plugins

use chrono::{DateTime, Utc};
use clap::{ArgMatches, Command};
use memflow_registry_client::shared::PluginVariant;

use crate::{
    commands::{plugin_extension, plugins_path},
    error::Result,
};

#[inline]
pub fn metadata() -> Command {
    Command::new("plugins")
        .subcommand_required(true)
        .subcommands([Command::new("list").alias("ls"), Command::new("purge")])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            // identical to print_plugin_versions_header() // TODO: restructure
            println!(
                "{0: <16} {1: <16} {2: <16} {3: <16} {4: <32} {5}",
                "NAME", "VERSION", "PLUGIN_VERSION", "DIGEST", "CREATED", "DOWNLOADED"
            );
            let paths = std::fs::read_dir(plugins_path())?;
            for path in paths.filter_map(|p| p.ok()) {
                if let Some(extension) = path.path().extension() {
                    if extension.to_str().unwrap_or_default() == "meta" {
                        if let Ok(metadata) = serde_json::from_str::<PluginVariant>(
                            &tokio::fs::read_to_string(path.path()).await?,
                        ) {
                            let file_metadata = tokio::fs::metadata(path.path()).await?;
                            let datetime: DateTime<Utc> = file_metadata.created()?.into();
                            println!(
                                "{0: <16} {1: <16} {2: <16} {3: <16} {4: <32} {5:}",
                                metadata.descriptor.name,
                                metadata.descriptor.version,
                                metadata.descriptor.plugin_version,
                                &metadata.digest[..7],
                                metadata.created_at.to_string(),
                                datetime.naive_utc().to_string(),
                            );
                        } else {
                            // TODO: print warning about orphaned plugin and give hints
                            // on how to install plugins from source with memflowup
                        }
                    }
                }
            }
        }
        Some(("purge", _)) => {
            // TODO: find and clean all files that have no .meta file
            // TODO: allow purging of everything
            let orphaned = remove_orphaned_plugins().await?;
            println!(
                "{} Plugins purged, removed {} plugins.",
                console::style("[=]").bold().dim().green(),
                orphaned,
            );
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Removes all plugins which do not have a proper .meta file associated with them.
async fn remove_orphaned_plugins() -> Result<usize> {
    let mut orphaned_plugins = 0;

    let paths = std::fs::read_dir(plugins_path())?;
    for path in paths.filter_map(|p| p.ok()) {
        if let Some(extension) = path.path().extension() {
            // TODO: should we only check for plugin_extension here?
            if extension.to_str().unwrap_or_default() == plugin_extension() {
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
                    println!(
                        "{} Deleting orphaned plugin: {:?} ({})",
                        console::style("[=]").bold().dim().green(),
                        path.path().as_os_str(),
                        reason
                    );

                    // remove plugin
                    tokio::fs::remove_file(path.path()).await?;

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

                    orphaned_plugins += 1;
                    continue;
                }
            }
        }
    }

    Ok(orphaned_plugins)
}
