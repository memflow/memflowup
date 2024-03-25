//! Clap subcommand to list all installed plugins

use chrono::{DateTime, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command};
use memflow_registry_client::shared::PluginVariant;

use crate::{
    commands::plugins_path,
    error::{Error, Result},
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
                "{0: <16} {1: <16} {2: <16} {3: <32} {4}",
                "NAME", "VERSION", "DIGEST", "CREATED", "DOWNLOADED"
            );
            let paths = std::fs::read_dir(plugins_path())?;
            for path in paths.filter_map(|p| p.ok()) {
                if let Some(extension) = path.path().extension() {
                    if extension.to_str().unwrap_or_default() == "meta" {
                        if let Ok(metadata) = serde_json::from_str::<PluginVariant>(
                            &std::fs::read_to_string(path.path())?,
                        ) {
                            let file_metadata = tokio::fs::metadata(path.path()).await?;
                            let datetime: DateTime<Utc> = file_metadata.created()?.into();
                            println!(
                                "{0: <16} {1: <16} {2: <16} {3: <32} {4:}",
                                metadata.descriptor.name,
                                metadata.descriptor.version,
                                &metadata.digest[..7],
                                metadata.created_at.to_string(),
                                datetime.naive_utc().to_string(),
                            );
                        } else {
                            // warn
                        }
                    }
                }
            }
        }
        Some(("purge", _)) => {
            println!("removing all plugins but the latest installed version");
            // TODO: find and clean all files that have no .meta file
            // TODO: allow purging of everything
        }
        _ => unreachable!(),
    }

    Ok(())
}
