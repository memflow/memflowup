use crate::{github_api, util};

use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use log::{debug, error, info, warn};

use rhai::{Dynamic, Engine, EvalAltResult, RegisterFn, RegisterResultFn, Scope};

use zip::ZipArchive;

/// Prints info
fn info(s: &str) -> () {
    info!("{}", s);
}

/// Prints an error
fn error(s: &str) -> () {
    error!("{}", s);
}

/// Returns the temp directory
fn tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

/// Returns the path to a temporary file with the given name
fn tmp_file(name: &str) -> String {
    env::temp_dir().join(name).to_string_lossy().to_string()
}

/// Downloads the given url to the destination file
fn download_file(url: &str, file: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    info!("download file from '{}' to '{}'", url.clone(), file.clone());
    let bytes = match util::download_file(url) {
        Ok(b) => b,
        Err(err) => {
            error!("{}", err);
            return Err(err.into());
        }
    };

    match fs::write(file, bytes) {
        Ok(()) => Ok(().into()),
        Err(err) => {
            error!("{}", err);
            Err(err.to_string().into())
        }
    }
}

// TODO:
// - function that allows downloading an entire git repository
// - git submodule manipulation
// - cargo command
// - make command
fn download_zip(url: &str, folder: &str, strip_path: i64) -> Result<Dynamic, Box<EvalAltResult>> {
    info!(
        "download zip file from '{}' to '{}'",
        url.clone(),
        folder.clone()
    );
    let bytes = match util::download_file(url) {
        Ok(b) => b,
        Err(err) => {
            error!("{}", err);
            return Err(err.into());
        }
    };

    let zip_cursor = std::io::Cursor::new(&bytes[..]);
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
                    PathBuf::from(folder).join(
                        file_path
                            .iter()
                            .skip(strip_path as usize)
                            .collect::<PathBuf>(),
                    )
                } else {
                    PathBuf::from(folder).join(file_path)
                };

                if file.is_dir() {
                    fs::create_dir_all(out_path).ok();
                } else {
                    debug!("extracing file {:?}", out_path);
                    let mut outfile = File::create(&out_path).expect("unable to write output file");
                    io::copy(&mut file, &mut outfile).expect("unable to write output file");
                }
            } else {
                warn!("invalid path in zip file for file: {:?}", file.name());
            }
        }
    }

    Ok(().into())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn find_platform_asset<'a>(release: &'a github_api::Release) -> Option<&'a github_api::Asset> {
    release.assets.iter().find(|a| a.name.ends_with(".so"))
}

#[cfg(target_os = "windows")]
fn find_platform_asset<'a>(release: &'a github_api::Release) -> Option<&'a github_api::Asset> {
    release.assets.iter().find(|a| a.name.ends_with(".dll"))
}

#[cfg(target_os = "macos")]
fn find_platform_asset<'a>(release: &'a github_api::Release) -> Option<&'a github_api::Asset> {
    release.assets.iter().find(|a| a.name.ends_with(".dylib"))
}

fn download_repository_binary(
    group: &str,
    repository: &str,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let releases: Vec<github_api::Release> = util::get_response(&format!(
        "https://api.github.com/repos/{}/{}/releases",
        group, repository
    ))?;

    match releases.iter().find(|r| !r.draft && !r.prerelease) {
        Some(release) => {
            info!(
                "latest stable release: {} (tag: {})",
                release.name, release.tag_name
            );
            match find_platform_asset(release) {
                Some(asset) => {
                    info!("valid binary found for current platform: {}", asset.name);
                    println!("yolo: {:?}", asset);
                    download_file(&asset.browser_download_url, &tmp_file(&asset.name))
                },
                None => {
                    Err(format!("unable to find appropiate binary for the current platform for release {}/{}/{}", group, repository, release.tag_name).into())
                }
            }
        }
        None => Err(format!("unable to find a release for {}/{}", group, repository).into()),
    }
}

// TODO: global configuration for binary / source / tag or branch
fn download_repository(group: &str, repository: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let download_binary = true;

    info!("downloading repository {}/{}", group, repository);

    if download_binary {
        return download_repository_binary(group, repository);
    } else {
        // select appropiate version and download
    }

    /*
    info!(
        "download zip file from '{}' to '{}'",
        url.clone(),
        folder.clone()
    );
    let bytes = match util::download_file(url) {
        Ok(b) => b,
        Err(err) => {
            error!("{}", err);
            return Err(err.into());
        }
    };

    let zip_cursor = std::io::Cursor::new(&bytes[..]);
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
                    PathBuf::from(folder).join(
                        file_path
                            .iter()
                            .skip(strip_path as usize)
                            .collect::<PathBuf>(),
                    )
                } else {
                    PathBuf::from(folder).join(file_path)
                };

                if file.is_dir() {
                    fs::create_dir_all(out_path).ok();
                } else {
                    debug!("extracing file {:?}", out_path);
                    let mut outfile = File::create(&out_path).expect("unable to write output file");
                    io::copy(&mut file, &mut outfile).expect("unable to write output file");
                }
            } else {
                warn!("invalid path in zip file for file: {:?}", file.name());
            }
        }
    }

    Ok(().into())
    */

    Ok(().into())
}

/// Executes cargo with the given flags
fn cargo(args: &str, pwd: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    info!("executing 'cargo {}' in '{}'", args.clone(), pwd.clone());

    let mut cmd = Command::new("cargo");

    cmd.current_dir(pwd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    for arg in args.split(" ") {
        cmd.arg(arg);
    }

    match cmd.output() {
        Ok(_) => Ok(().into()),
        Err(err) => {
            error!("{}", err);
            return Err(err.to_string().into());
        }
    }
}

/// Copies a file from 'from' to 'to'
fn copy_file(from: &str, to: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    // std::fs::create_dir_all(memflow_user_path.clone()).ok(); // TODO: handle file exists error and clean folder
    match std::fs::copy(from, to) {
        Ok(_) => Ok(().into()),
        Err(err) => {
            error!("{}", err);
            return Err(err.to_string().into());
        }
    }
}

fn install_connector(from: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let in_path = PathBuf::from(from);
    {
        let user_dir = dirs::home_dir()
            .unwrap()
            .join(".local")
            .join("lib")
            .join("memflow");
        let out_path = user_dir.join(in_path.file_name().unwrap());

        info!(
            "copying '{:?}' to '{:?}'",
            in_path.clone(),
            out_path.clone()
        );
        match std::fs::copy(in_path.clone(), out_path.clone()) {
            Ok(_) => (),
            Err(err) => {
                error!("{}", err);
                return Err(err.to_string().into());
            }
        };
    }

    // TODO: check system install
    {
        let system_dir = PathBuf::from("/").join("usr").join("lib").join("memflow");
        let out_path = system_dir.join(in_path.file_name().unwrap());

        info!(
            "copying '{:?}' to '{:?}'",
            in_path.clone(),
            out_path.clone()
        );
        match Command::new("sudo")
            .arg("cp")
            .arg(in_path.clone())
            .arg(out_path.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
        {
            Ok(_) => (),
            Err(err) => {
                error!("{}", err);
                return Err(err.to_string().into());
            }
        };
    }
    Ok(().into())
}

// 1. we need to query all available branches
// https://api.github.com/repos/memflow/memflow-coredump/branches

// if we want to install a stable version of the connector
// we should pick the latest tag or master branch via this api:
// https://api.github.com/repos/memflow/memflow-coredump/tags
// this will also give us a zip file in the zipball_url / tarball_url

// if we want to install a specific branch just download the branch
// specific branches can then be downloaded as follows:
// https://github.com/memflow/memflow-coredump/archive/master.tar.gz

pub fn execute<P: AsRef<Path>>(path: P) -> () {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(999, 999);

    engine
        .register_fn("info", info)
        .register_fn("error", error)
        .register_fn("tmp_dir", tmp_dir)
        .register_fn("tmp_file", tmp_file)
        .register_fn("tmp_folder", tmp_file)
        .register_result_fn("download_file", download_file)
        .register_result_fn("download_zip", download_zip)
        .register_result_fn("download_repository", download_repository)
        .register_result_fn("cargo", cargo)
        .register_result_fn("copy_file", copy_file)
        .register_result_fn("install_connector", install_connector);

    let mut scope = Scope::new();
    let ast = engine
        .compile_file_with_scope(&mut scope, path.as_ref().into())
        .unwrap();
    let _: () = engine.eval_ast_with_scope(&mut scope, &ast).unwrap();

    let _: () = engine.call_fn(&mut scope, &ast, "build", ()).unwrap();

    let _: () = engine.call_fn(&mut scope, &ast, "install", ()).unwrap();

    //Ok(())
}
