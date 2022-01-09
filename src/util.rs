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

pub fn user_input_boolean(question: &str, default: bool) -> crate::Result<bool> {
    user_input(question, &["y", "n"], !default as usize).map(|r| r == 0)
}

pub fn user_input(question: &str, options: &[&str], default: usize) -> crate::Result<usize> {
    loop {
        print!("{}", question);

        print!(" [");

        for (i, option) in options.iter().enumerate() {
            if i != 0 {
                print!("/");
            }

            if i == default {
                print!("{}", option.to_uppercase());
            } else {
                print!("{}", option);
            }
        }

        print!("]: ");

        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");
        let input_stripped = input.trim().to_lowercase();

        if input_stripped.is_empty() {
            return Ok(default);
        } else {
            let mut iter = options
                .iter()
                .enumerate()
                .filter(|(_, o)| o.to_lowercase().starts_with(&input_stripped));

            // Return only if there's just a single starts_with match
            match (iter.next(), iter.next()) {
                (Some((i, _)), None) => return Ok(i),
                _ => {}
            }
        }
    }
}

/// Returns the path to a temporary file with the given name
#[allow(unused)]
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

pub fn create_dir_with_elevation(path: impl AsRef<Path>, elevate: bool) -> crate::Result<()> {
    match std::fs::create_dir_all(&path) {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied && elevate {
                let path_str = path
                    .as_ref()
                    .to_str()
                    .ok_or("directory contains invalid characters!")?;
                info!("Elevated mkdir {}", path_str);
                match Command::new("sudo")
                    .args(&["mkdir", "-p", path_str])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        error!("{}", err);
                        Err(err.into())
                    }
                }
            } else {
                Err(err.into())
            }
        }
    }
}

pub fn write_with_elevation(
    path: impl AsRef<Path>,
    elevate: bool,
    handler: impl Fn(std::fs::File) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::File::create(&path) {
        Ok(file) => handler(file),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied && elevate {
                let dir = make_temp_dir("memflowup_build", &[&path.as_ref().to_string_lossy()])?;
                let tmp_path = dir.join("tmp_file");
                let file = std::fs::File::create(&tmp_path)?;
                handler(file)?;
                copy_file(&tmp_path, &path, true).map_err(Into::into)
            } else {
                Err(err.into())
            }
        }
    }
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

pub fn config_dir(system_wide: bool) -> PathBuf {
    if system_wide {
        #[cfg(not(windows))]
        {
            "/etc/memflowup/".into()
        }
        #[cfg(windows)]
        // TODO: pick a better path
        {
            "C:\\memflowup\\{}".into()
        }
    } else {
        let mut path = dirs::config_dir().unwrap();
        path.push("memflowup");
        path
    }
}

#[allow(unused)]
pub fn executable_dir(system_wide: bool) -> PathBuf {
    if system_wide {
        #[cfg(not(windows))]
        {
            "/usr/local/bin/".into()
        }
        #[cfg(windows)]
        // TODO: pick a better path
        {
            "C:\\memflowup\\{}".into()
        }
    } else {
        #[cfg(not(windows))]
        {
            let mut path = dirs::executable_dir().unwrap();
            path.push("memflowup");
            path
        }
        #[cfg(windows)]
        {
            panic!("windows does not have a non system-wide program directory")
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
