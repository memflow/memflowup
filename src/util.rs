use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use log::{debug, error, info, warn};

use pbr::ProgressBar;
use progress_streams::ProgressReader;
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use crc::{Crc, CRC_64_GO_ISO};

const CRC: Crc<u64> = Crc::<u64>::new(&CRC_64_GO_ISO);

pub fn user_input_boolean(question: &str, default: bool) -> bool {
    loop {
        print!("{}", question);
        if default {
            print!(" [Y/n]: ");
        } else {
            print!(" [y/N]: ");
        }
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");
        let input_stripped = input.strip_suffix('\n').unwrap_or_default().to_lowercase();

        if input_stripped.is_empty() {
            return default;
        } else if input_stripped.starts_with('y') {
            return true;
        } else if input_stripped.starts_with('n') {
            return false;
        }
    }
}

/// Returns the path to a temporary file with the given name
pub fn tmp_file(name: &str) -> String {
    env::temp_dir().join(name).to_string_lossy().to_string()
}

/// Queries the URL and returns the deserialized response.
pub fn http_get_json<T: DeserializeOwned>(url: &str) -> Result<T, &'static str> {
    let resp = ureq::get(url).call();
    if !resp.ok() {
        return Err("unable to download file");
    }

    let mut reader = resp.into_reader();

    let mut response = String::new();
    reader
        .read_to_string(&mut response)
        .map_err(|_| "unable to read from http request")?;

    serde_json::from_str(&response).map_err(|_| "unable to deserialize http response")
}

/// Downloads the specified file and returns a byte buffer containing the data.
pub fn http_download_file(url: &str) -> Result<Vec<u8>, &'static str> {
    info!("downloading file from {}", url);
    let resp = ureq::get(url).call();
    if !resp.ok() {
        return Err("unable to download file");
    }

    let buffer = if resp.has("Content-Length") {
        let len = resp
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap();

        let mut reader = resp.into_reader();
        let buffer = read_to_end(&mut reader, len)?;
        assert_eq!(buffer.len(), len);
        buffer
    } else {
        let mut buffer = Vec::new();
        let mut reader = resp.into_reader();
        reader
            .read_to_end(&mut buffer)
            .map_err(|_| "unable to read from http request")?;
        buffer
    };

    Ok(buffer)
}

fn read_to_end<T: Read>(reader: &mut T, len: usize) -> Result<Vec<u8>, &'static str> {
    let mut buffer = vec![];

    let total = Arc::new(AtomicUsize::new(0));
    let mut reader = ProgressReader::new(reader, |progress: usize| {
        total.fetch_add(progress, Ordering::SeqCst);
    });
    let mut pb = ProgressBar::new(len as u64);

    let finished = Arc::new(AtomicBool::new(false));
    let thread = {
        let finished_thread = finished.clone();
        let total_thread = total.clone();

        std::thread::spawn(move || {
            while !finished_thread.load(Ordering::Relaxed) {
                pb.set(total_thread.load(Ordering::SeqCst) as u64);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            pb.finish();
        })
    };

    reader
        .read_to_end(&mut buffer)
        .map_err(|_| "unable to read from http request")?;
    finished.store(true, Ordering::Relaxed);
    thread.join().unwrap();

    Ok(buffer)
}

pub fn zip_unpack(in_buf: &[u8], out_dir: &PathBuf, strip_path: i64) -> Result<(), String> {
    let zip_cursor = std::io::Cursor::new(&in_buf[..]);
    let mut zip_archive = match ZipArchive::new(zip_cursor) {
        Ok(archive) => archive,
        Err(err) => {
            error!("{:?}", err);
            return Err(format!("{:?}", err).into());
        }
    };

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

/// Copies a file from 'from' to 'to'
pub fn copy_file(
    from: &impl AsRef<Path>,
    to: &impl AsRef<Path>,
    elevate_if_needed: bool,
) -> Result<(), std::io::Error> {
    info!(
        "copying '{:?}' to '{:?}'",
        from.as_ref().to_string_lossy(),
        to.as_ref().to_string_lossy(),
    );

    // std::fs::create_dir_all(memflow_user_path.clone()).ok(); // TODO: handle file exists error and clean folder
    match std::fs::copy(from, to) {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied && elevate_if_needed {
                info!(
                    "Elevated copy {} to {}",
                    from.as_ref().to_string_lossy(),
                    to.as_ref().to_string_lossy()
                );
                match Command::new("sudo")
                    .arg("cp")
                    .arg(from.as_ref().to_str().ok_or_else(|| {
                        std::io::Error::new(err.kind(), "from path contains invalid characters!")
                    })?)
                    .arg(to.as_ref().to_str().ok_or_else(|| {
                        std::io::Error::new(err.kind(), "to path contains invalid characters!")
                    })?)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        error!("{}", err);
                    }
                };
            }
            error!("{}", &err);
            return Err(err);
        }
    }
}

/// Create a temporary directory, but it can already be an existing one.
pub fn make_temp_dir(subdir: &str, names: &[&str]) -> Result<TempDir, std::io::Error> {
    let tmp_dir = std::env::temp_dir();

    let mut digest = CRC.digest();

    for n in names {
        digest.update(n.as_bytes());
    }

    let tmp_path = tmp_dir.join(&format!("{}/{}", subdir, digest.finalize()));
    std::fs::create_dir_all(&tmp_path)?;

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
