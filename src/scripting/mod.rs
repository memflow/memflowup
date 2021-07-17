use crate::{database::EntryType, github_api, package::Package, util};

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use log::*;

use rhai::{Dynamic, Engine, EvalAltResult, Scope};

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
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        format!("lib{}.so", name)
    }
    #[cfg(target_os = "macos")]
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
    system_wide: bool,
    installed: &'a RefCell<Vec<String>>,
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

    fn clone_repository(&mut self) -> Result<String, Box<EvalAltResult>> {
        let mut path: PathBuf = self.tmp_dir.as_path().into();

        let dir = format!("{:x}", CRC.checksum(self.sha.as_bytes()));

        path.push(dir);

        let path_str = path.to_str().ok_or("invalid path generated")?;

        Command::new("git")
            .args(&[
                "clone",
                "--single-branch",
                "--recursive",
                "--depth",
                "1",
                &self.package.repo_root_url,
                path_str,
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|_| "unable to clone repository")?;

        Ok(path_str.into())
    }

    fn dkms_install(&mut self, path: String) -> Result<(), Box<EvalAltResult>> {
        Command::new("sudo")
            .args(&["dkms", "install", &path])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output().map_err(|_| "unable to execute DKMS add. DKMS is only available on *nix based systems (but not macOS)")?;

        Ok(())
    }

    fn dkms_install_tarball(&mut self, bytes: Vec<u8>) -> Result<(), Box<EvalAltResult>> {
        let mut path: PathBuf = self.tmp_dir.as_path().into();

        let dir = format!("{:x}.tar.dz", CRC.checksum(self.sha.as_bytes()));

        path.push(dir);

        let path_str = path.to_str().ok_or("invalid path generated")?;

        {
            let mut file = File::create(&path).map_err(|_| "failed to create tarball file")?;
            file.write_all(&bytes)
                .map_err(|_| "failed to write the tarball")?;
        }

        Command::new("sudo")
            .args(&["dkms", "install", &format!("--archive={}", path_str)])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output().map_err(|_| "unable to execute DKMS add. DKMS is only available on *nix based systems (but not macOS)")?;

        Ok(())
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

        let out_path = if !self.system_wide {
            let user_dir = dirs::home_dir()
                .unwrap()
                .join(".local")
                .join("lib")
                .join("memflow");
            user_dir.join(&out_filename)
        } else {
            let system_dir = PathBuf::from("/").join("usr").join("lib").join("memflow");
            system_dir.join(out_filename)
        };

        out_path.to_str().ok_or("invalid output path")?;

        util::copy_file(&in_path, &out_path, self.system_wide)
            .map_err(|_| "failed to copy to user path")?;

        self.installed
            .try_borrow_mut()
            .unwrap()
            .push(out_path.to_str().unwrap().to_string());

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
    system_wide: bool,
    entrypoint: &str,
) -> Result<(EntryType, Vec<String>), Box<dyn std::error::Error>> {
    let tmp_dir = util::make_temp_dir(
        "memflowup_build",
        &[
            &package.name,
            entrypoint,
            package.branch(dev_branch).unwrap(),
        ],
    )?;

    let installed = RefCell::new(vec![]);
    let installed_release = RefCell::new(None);

    let branch =
        github_api::get_branch(&package.repo_root_url, package.branch(dev_branch).unwrap())?;

    let ctx = ScriptCtx {
        package,
        tmp_dir: &tmp_dir,
        dev_branch,
        system_wide,
        installed: &installed,
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
        .register_result_fn("clone_repository", ScriptCtx::clone_repository)
        .register_result_fn("extract", ScriptCtx::extract)
        .register_fn("crate_name", ScriptCtx::crate_name)
        .register_result_fn(
            "copy_cargo_plugin_artifact",
            ScriptCtx::copy_cargo_plugin_artifact,
        )
        .register_result_fn("dkms_install", ScriptCtx::dkms_install)
        .register_result_fn("dkms_install", ScriptCtx::dkms_install_tarball)
        .register_fn("info", info)
        .register_fn("error", error)
        .register_fn("name_to_lib", name2lib)
        .register_result_fn("cargo", cargo);

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
        installed.into_inner(),
    ))
}
