use std::cmp::Reverse;
use std::fs::{self, File};
use std::io::{self};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, warn};
use memflow_registry_client::shared::PluginVariant;
use reqwest::Response;
use zip::ZipArchive;

use crate::error::Result;

/// Returns the path in which memflow plugins are stored.
///
/// On unix this is returns ~/.local/lib/memflow
/// On windows this returns C:\Users\[Username]\Documents\memflow
pub(crate) fn plugins_path() -> PathBuf {
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

// TODO: move this to utils
/// Returns the path in which memflowup config is stored.
pub(crate) fn config_path() -> PathBuf {
    if cfg!(unix) {
        dirs::home_dir().unwrap().join(".config").join("memflowup")
    } else {
        dirs::document_dir().unwrap()
    }
}

/// Returns the path that points to the memflowup config.
#[inline]
pub(crate) fn config_file_path() -> PathBuf {
    config_path().join("config.json")
}

/// Constructs the filename of this plugin for the current os.
///
/// On unix this returns libmemflow_[name]_[digest].so/.dylib
/// On windows this returns memflow_[name]_[digest].dll
pub(crate) fn plugin_file_name(variant: &PluginVariant) -> PathBuf {
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
    file_name.set_extension(plugin_extension());

    file_name
}

/// Returns the plugin extension appropriate for the current os
pub(crate) fn plugin_extension() -> &'static str {
    #[cfg(target_os = "windows")]
    return "dll";
    #[cfg(target_os = "linux")]
    return "so";
    #[cfg(target_os = "macos")]
    return "dylib";
}

pub async fn read_response_with_progress(response: Response) -> Result<Bytes> {
    let mut buffer = BytesMut::new();
    if let Some(content_length) = response.content_length() {
        let pb = ProgressBar::new(content_length);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                    .unwrap()
                    .progress_chars("#>-"));

        // download data in chunks to show progress
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(chunk.as_ref());
            pb.inc(chunk.len() as u64);
        }
        pb.finish();
    } else {
        // no content-length set, fallback without progress bar
        warn!("skipping progress bar because content-length is not set");
        buffer.extend_from_slice(&response.bytes().await?.to_vec()[..]);
    }
    Ok(buffer.freeze())
}

/// Returns a list of all local plugins with their .meta information attached (sorted in the same way as memflow-registry)
pub async fn local_plugins() -> Result<Vec<(PathBuf, PluginVariant)>> {
    let mut result = Vec::new();

    let paths = std::fs::read_dir(plugins_path())?;
    for path in paths.filter_map(|p| p.ok()) {
        if let Some(extension) = path.path().extension() {
            if extension.to_str().unwrap_or_default() == "meta" {
                if let Ok(metadata) = serde_json::from_str::<PluginVariant>(
                    &tokio::fs::read_to_string(path.path()).await?,
                ) {
                    // TODO: additionally check existence of the file name and pass it over
                    result.push((path.path(), metadata));
                } else {
                    // TODO: print warning about orphaned plugin and give hints
                    // on how to install plugins from source with memflowup
                }
            }
        }
    }

    // sort by plugin_name, plugin_version and created_at
    result.sort_by_key(|(_, variant)| {
        (
            variant.descriptor.name.clone(),
            Reverse(variant.descriptor.plugin_version),
            Reverse(variant.created_at),
        )
    });

    Ok(result)
}

/// Unpack zip archive in memory
pub fn zip_unpack(in_buf: &[u8], out_dir: &Path, strip_path: i64) -> crate::Result<()> {
    let zip_cursor = std::io::Cursor::new(in_buf);
    let mut zip_archive = ZipArchive::new(zip_cursor)?;

    for i in 0..zip_archive.len() {
        if let Ok(mut file) = zip_archive.by_index(i) {
            if let Some(file_path) = file.enclosed_name() {
                let out_path = if strip_path > 0 {
                    PathBuf::from(out_dir).join(
                        file_path
                            .iter()
                            .skip(strip_path as usize)
                            .collect::<PathBuf>(),
                    )
                } else {
                    PathBuf::from(out_dir).join(file_path)
                };

                if file.is_dir() {
                    fs::create_dir_all(out_path).ok();
                } else {
                    debug!("extracting file {:?}", out_path);
                    match File::create(&out_path) {
                        Ok(mut outfile) => match io::copy(&mut file, &mut outfile) {
                            Ok(_) => {
                                info!(
                                    "successfuly extracted file to {}",
                                    out_path.to_string_lossy()
                                );
                            }
                            Err(err) => {
                                warn!("skipping unzip to {}: {}", out_path.to_string_lossy(), err);
                            }
                        },
                        Err(err) => {
                            warn!("skipping unzip to {}: {}", out_path.to_string_lossy(), err);
                        }
                    }
                }
            } else {
                warn!("invalid path in zip file for file: {:?}", file.name());
            }
        }
    }

    Ok(())
}

/// Executes cargo with the given flags
pub fn cargo<P: AsRef<Path>>(args: &str, pwd: P) -> Result<Output> {
    log::info!("executing 'cargo {}' in {:?}", args, pwd.as_ref());
    let mut cmd = Command::new("cargo");

    cmd.current_dir(pwd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    for arg in args.split(' ') {
        cmd.arg(arg);
    }

    let output = cmd.output()?;
    Ok(output)
}

/// Create a temporary directory, but it can already be an existing one.
pub async fn create_temp_dir(subdir: &str, uid: &str) -> crate::Result<TempDir> {
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("{}/{}", subdir, uid));
    tokio::fs::create_dir_all(&tmp_path).await?;
    Ok(TempDir(tmp_path))
}

pub struct TempDir(PathBuf);

impl std::ops::Deref for TempDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for TempDir {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Debug for TempDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("cannot delete the tmp dir")
    }
}
