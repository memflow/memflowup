use crate::{database::EntryType, github_api, package::Package, util};

use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use log::{debug, error, info, warn};

use rhai::{Dynamic, Engine, EvalAltResult, RegisterFn, RegisterResultFn, Scope};

use zip::ZipArchive;

use std::cell::RefCell;

use crc::{Crc, CRC_64_GO_ISO};

const DEFAULT_SCRIPT: &str = include_str!("../../standard.rhai");

const CRC: Crc<u64> = Crc::<u64>::new(&CRC_64_GO_ISO);

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
    let bytes = match util::http_download_file(url) {
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

// TODO: global configuration for binary / source / tag or branch
fn download_repository(group: &str, repository: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let download_binary = true;

    info!("downloading repository {}/{}", group, repository);

    if download_binary {
        //return download_repository_binary(group, repository);
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

fn name2lib(name: &str) -> String {
    let name = name.replace("-", "_");
    #[cfg(unix)]
    {
        format!("lib{}.so", name)
    }
    #[cfg(macos)]
    {
        format!("lib{}.dylib", name)
    }
    #[cfg(windows)]
    {
        format!("{}.dll", name)
    }
}

#[derive(Clone, Copy)]
pub struct ScriptCtx<'a> {
    package: &'a Package,
    tmp_dir: &'a util::TempDir,
    dev_branch: bool,
    installed_local: &'a RefCell<Vec<String>>,
    installed_system: &'a RefCell<Vec<String>>,
    installed_release: &'a RefCell<Option<EntryType>>,
    sha: &'a str,
}

impl<'a> ScriptCtx<'a> {
    fn download_repository(&mut self) -> Result<Vec<u8>, Box<EvalAltResult>> {
        // TODO: support non-github repos
        let url = format!("{}/archive/{}.zip", self.package.repo_root_url, self.sha,);

        info!("download zip file from '{}'", &url,);

        util::http_download_file(&url).map_err(Into::into)
    }

    fn extract(&mut self, bytes: Vec<u8>) -> Result<String, Box<EvalAltResult>> {
        let mut path: PathBuf = self.tmp_dir.as_path().into();

        let dir = format!("{:x}", CRC.checksum(&bytes));

        path.push(&dir);

        info!("Extracting {:?}", path);
        util::zip_unpack(&bytes, &path, 1)?;

        Ok(path.to_str().ok_or("failed to extract")?.to_string())
    }

    fn crate_name(&mut self) -> String {
        self.package.name.clone()
    }

    fn copy_cargo_plugin_artifact(
        &mut self,
        path: &str,
        artifact: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        // Mark that we did source installation:
        *self.installed_release.try_borrow_mut().unwrap() =
            Some(EntryType::GitSource(self.sha.into()));

        let mut in_path = PathBuf::from(path);
        in_path.push("target");
        in_path.push("release");
        in_path.push(artifact);

        let stem = in_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("malformed filename")?;
        let extension = in_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or("malformed extension")?;
        let out_filename = format!(
            "{}.{}.{}",
            stem,
            if self.dev_branch { "dev" } else { "stable" },
            extension
        );

        {
            let user_dir = dirs::home_dir()
                .unwrap()
                .join(".local")
                .join("lib")
                .join("memflow");
            let out_path = user_dir.join(&out_filename);

            out_path.to_str().ok_or("invalid output path")?;

            util::copy_file(&in_path, &out_path, false)
                .map_err(|_| "failed to copy to user path")?;

            self.installed_local
                .try_borrow_mut()
                .unwrap()
                .push(out_path.to_str().unwrap().to_string());
        }

        // TODO: check system install
        {
            let system_dir = PathBuf::from("/").join("usr").join("lib").join("memflow");
            let out_path = system_dir.join(out_filename);

            out_path.to_str().ok_or("invalid output path")?;

            util::copy_file(&in_path, &out_path, true)
                .map_err(|_| "failed to copy to system path")?;

            self.installed_system
                .try_borrow_mut()
                .unwrap()
                .push(out_path.to_str().unwrap().to_string());
        }

        Ok(().into())
    }
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

/*pub fn execute_file<P: AsRef<Path>>(path: P) -> () {
}*/

pub fn execute_installer(
    package: &Package,
    dev_branch: bool,
    entrypoint: &str,
) -> Result<(EntryType, Vec<String>, Vec<String>), Box<dyn std::error::Error>> {
    let tmp_dir = util::make_temp_dir(
        "memflowup_build",
        &[
            &package.name,
            entrypoint,
            package.branch(dev_branch).unwrap(),
        ],
    )?;

    let installed_local = RefCell::new(vec![]);
    let installed_system = RefCell::new(vec![]);
    let installed_release = RefCell::new(None);

    let branch =
        github_api::get_branch(&package.repo_root_url, package.branch(dev_branch).unwrap())?;

    let ctx = ScriptCtx {
        package,
        tmp_dir: &tmp_dir,
        dev_branch,
        installed_local: &installed_local,
        installed_system: &installed_system,
        installed_release: &installed_release,
        sha: &branch.commit.sha,
    };

    let download_script = if let Some(path) = &package.install_script_path {
        Some(
            github_api::download_raw(&package.repo_root_url, &branch.commit.sha, &path)
                .map(|b| String::from_utf8_lossy(&b).to_string())?,
        )
    } else {
        None
    };

    let script = download_script.as_deref().unwrap_or(DEFAULT_SCRIPT);

    let mut engine = Engine::new();
    engine.set_max_expr_depths(999, 999);

    engine
        .register_type::<ScriptCtx>()
        .register_result_fn("download_repository", ScriptCtx::download_repository)
        .register_result_fn("extract", ScriptCtx::extract)
        .register_fn("crate_name", ScriptCtx::crate_name)
        .register_result_fn("copy_cargo_plugin_artifact", ScriptCtx::copy_cargo_plugin_artifact)
        .register_fn("info", info)
        .register_fn("error", error)
        .register_fn("name_to_lib", name2lib)
        //.register_fn("tmp_dir", tmp_dir)
        //        .register_fn("tmp_file", tmp_file)
        //      .register_fn("tmp_folder", tmp_file)
        //    .register_result_fn("download_file", download_file)
        //.register_result_fn("download_zip", download_zip)
        //.register_result_fn("download_repository", download_repository)
        .register_result_fn("cargo", cargo)
        //.register_result_fn("copy_file", copy_file)
        //.register_result_fn("install_connector", install_connector);
        ;

    let mut scope = Scope::new();
    let ast = engine.compile_with_scope(&mut scope, script).unwrap();

    // SAFETY: engine does not outlive the ctx/package.
    let ctx = unsafe { std::mem::transmute::<_, ScriptCtx<'static>>(ctx) };

    engine
        .call_fn(&mut scope, &ast, entrypoint, (ctx,))
        .map_err(|e| e.to_string())?;

    std::mem::drop(engine);

    Ok((
        installed_release.into_inner().ok_or_else(|| {
            // TODO: cleanup artifacts
            "Script failed to mark installed release!"
        })?,
        installed_local.into_inner(),
        installed_system.into_inner(),
    ))
}
