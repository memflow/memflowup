use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::warn;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    error::{Error, Result},
    registry::{self, PluginEntry, PluginUri},
};

#[inline]
pub fn metadata() -> Command {
    Command::new("pull").args([
        Arg::new("plugin_uri").required(true).action(ArgAction::Set),
        Arg::new("force")
            .short('f')
            .long("force")
            .help("forces download of the plugin even if it is already installed.")
            .action(ArgAction::SetTrue),
    ])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let plugin_uri = matches.get_one::<String>("plugin_uri").unwrap();
    let force = matches.get_flag("force");

    // find the correct plugin variant based on the input arguments
    let plugin_uri: PluginUri = plugin_uri.parse()?;
    let variant = registry::find_by_uri(&plugin_uri).await?;

    // query file
    let response = registry::download(&plugin_uri, &variant).await?;

    // check if file already exists
    let file_name = plugin_file_name(&variant);
    if !force && file_name.exists() {
        let bytes = tokio::fs::read(&file_name).await?;
        let digest = sha256::digest(&bytes[..]);

        // check if the plugin digest matches with the one from memflow-registry
        if variant.digest == digest {
            println!(
                "{} Plugin {:?} already exists with the same checksum, skipping download.",
                console::style("[X]").bold().dim().red(),
                file_name.file_name().unwrap()
            );
            return Err(Error::AlreadyExists("plugin already installed".to_owned()));
        } else {
            println!(
                "{} Plugin {:?} already exists with a different checksum, redownloading.",
                console::style("[X]").bold().dim().red(),
                file_name.file_name().unwrap()
            );
        }
    }

    // create the plugin file
    let mut file = File::create(&file_name).await?;

    println!(
        "{} Downloading plugin: {:?}",
        console::style("[-]").bold().dim(),
        file_name.file_name().unwrap()
    );

    // write the file
    if let Some(content_length) = response.content_length() {
        let pb = ProgressBar::new(content_length);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                    .unwrap()
                    .progress_chars("#>-"));

        // download data in chunks to show progress
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(chunk.as_ref()).await?;
            pb.inc(chunk.len() as u64);
        }
        pb.finish();
    } else {
        // no content-length set, fallback without progress bar
        warn!("skipping progress bar because content-length is not set");
        file.write_all(&response.bytes().await?.to_vec()[..])
            .await?;
    }
    file.flush().await?;

    // TODO: - download plugin to a temporary directory first
    // TODO: - then verify signature
    // TODO: - after signature verification copy plugin to final destination

    println!(
        "{} Downloaded plugin to: {:?}",
        console::style("[=]").bold().dim(),
        file_name.as_os_str(),
    );

    // store .meta file of plugin containing all relevant information
    // TODO: this does not contain all plugins in this file - allow querying that from memflow-registry as well
    let mut file_name = file_name.clone();
    file_name.set_extension("meta");
    tokio::fs::write(&file_name, serde_json::to_string_pretty(&variant)?).await?;

    println!(
        "{} Wrote plugin metadata to: {:?}",
        console::style("[=]").bold().dim(),
        file_name.as_os_str(),
    );

    Ok(())
}

/// Returns the path in which memflow plugins are stored.
///
/// On unix this is returns ~/.local/lib/memflow
/// On windows this returns C:\Users\[Username]\Documents\memflow
fn plugins_path() -> PathBuf {
    if cfg!(unix) {
        dirs::home_dir()
            .unwrap()
            .join(".local")
            .join("lib")
            .join("memflow")
    } else {
        dirs::document_dir().unwrap().join("memflow")
    }
}

/// Constructs the filename of this plugin for the current os.
///
/// On unix this returns libmemflow_[name]_[digest].so/.dylib
/// On windows this returns memflow_[name]_[digest].dll
fn plugin_file_name(variant: &PluginEntry) -> PathBuf {
    let mut file_name = plugins_path();

    // prepend the library name and append the file digest
    if cfg!(unix) {
        file_name.push(&format!(
            "libmemflow_{}_{}",
            variant.descriptor.name,
            &variant.digest[..7]
        ))
    } else {
        file_name.push(&format!(
            "memflow_{}_{}",
            variant.descriptor.name,
            &variant.digest[..7]
        ))
    }

    // append appropriate file extension
    #[cfg(target_os = "windows")]
    file_name.set_extension("dll");
    #[cfg(target_os = "linux")]
    file_name.set_extension("so");
    #[cfg(target_os = "macos")]
    file_name.set_extension("dylib");

    file_name
}
