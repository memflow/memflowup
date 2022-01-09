use crate::{
    database::{load_database, Branch, EntryType},
    github_api,
    package::{Package, PackageOpts},
    util,
};

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

fn name2lib_with_arch(name: &str) -> String {
    name2lib(&format!("{}.{}", name, arch_str()))
}

macro_rules! cfg_arch {
    ($arch:expr) => {
        #[cfg(target_arch = $arch)]
        {
            return $arch;
        }
    };
}

fn arch_str() -> &'static str {
    cfg_arch!("x86_64");
    cfg_arch!("x86");
    cfg_arch!("aarch64");
    cfg_arch!("arm");
    cfg_arch!("wasm32");
    cfg_arch!("mips64");
    cfg_arch!("mips");
    cfg_arch!("powerpc64");
    cfg_arch!("powerpc");
    cfg_arch!("nvptx");
}

#[derive(Clone, Copy)]
pub struct ScriptCtx<'a> {
    package: &'a Package,
    opts: &'a PackageOpts,
    tmp_dir: &'a PathBuf,
    branch: Branch,
    installed: &'a RefCell<Vec<String>>,
    installed_release: &'a RefCell<Option<EntryType>>,
    sha: &'a str,
}

impl<'a> ScriptCtx<'a> {
    fn download_repository(&mut self) -> Result<Vec<u8>, Box<EvalAltResult>> {
        // TODO: support non-github repos
        let url = format!("{}/archive/{}.zip", self.package.repo_root_url, self.sha);

        info!("download zip file from '{}'", &url);

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
                "-b",
                self.package.branch(self.branch).unwrap(),
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

    fn build_path(&mut self) -> String {
        self.tmp_dir.to_str().unwrap().to_string()
    }

    fn entry_type(&self) -> EntryType {
        if self.opts.is_local {
            EntryType::LocalPath(self.package.repo_root_url.clone())
        } else {
            EntryType::GitSource(self.sha.into())
        }
    }

    fn copy_cargo_plugin_artifact(
        &mut self,
        path: &str,
        artifact: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        // Mark that we did source installation:
        *self.installed_release.try_borrow_mut().unwrap() = Some(self.entry_type());

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

        let out_filename = if self.opts.is_local {
            format!("{}.{}.{}", stem, arch_str(), extension)
        } else {
            format!("{}.{}.{}", stem, self.branch.filename(), extension)
        };

        if self.opts.nocopy {
            info!("{}", out_filename);
        } else {
            let out_dir = self.package.ty.install_path(self.opts.system_wide);
            let out_path = out_dir.join(out_filename);

            out_path.to_str().ok_or("invalid output path")?;

            util::create_dir_with_elevation(&out_dir.as_path(), self.opts.system_wide)
                .map_err(|_| format!("unable to create plugin target directory: {:?}", out_dir))?;

            util::copy_file(&in_path, &out_path, self.opts.system_wide).map_err(|_| {
                format!("unable to copy plugin from {:?} to {:?}", in_path, out_path)
            })?;

            info!("successfully copied {:?} to {:?}", in_path, out_path);

            self.installed
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
    opts: &PackageOpts,
    branch: Branch,
    entrypoint: &str,
) -> Result<(EntryType, Vec<String>), Box<dyn std::error::Error>> {
    // if local, use package path
    let (tmp_dir, local_dir) = if opts.is_local {
        (None, Some(PathBuf::from(&package.repo_root_url)))
    } else {
        (
            Some(util::make_temp_dir(
                "memflowup_build",
                &[&package.name, entrypoint, package.branch(branch).unwrap()],
            )?),
            None,
        )
    };

    let tmp_dir = tmp_dir.as_deref().or(local_dir.as_ref()).unwrap();

    let installed = RefCell::new(vec![]);
    let installed_release = RefCell::new(None);

    // if local, use LOCAL hash
    let sha = if opts.is_local {
        "LOCAL".to_string()
    } else {
        let branch =
            github_api::get_branch(&package.repo_root_url, package.branch(branch).unwrap())?;
        branch.commit.sha
    };

    let ctx = ScriptCtx {
        package,
        opts,
        tmp_dir,
        branch,
        installed: &installed,
        installed_release: &installed_release,
        sha: &sha,
    };

    // check if we have the latest version installed
    if !opts.reinstall {
        let db = load_database(branch, opts.system_wide)?;
        if let Some(p) = db.get(&package.name) {
            if p.ty == ctx.entry_type() {
                println!(
                    "The installed version of {} is already the latest version.",
                    &package.name
                );
                return Ok((p.ty.clone(), p.artifacts.clone()));
            }
        }
    }

    // if local, do not download the script
    let download_script = if let Some(path) = &package.install_script_path {
        if !opts.is_local {
            Some(
                github_api::download_raw(&package.repo_root_url, &sha, &path)
                    .map(|b| String::from_utf8_lossy(&b).to_string())?,
            )
        } else {
            Some({
                let mut path_buf = tmp_dir.clone();
                path_buf.push(&path);
                std::fs::read_to_string(path_buf)?
            })
        }
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
        .register_fn("build_path", ScriptCtx::build_path)
        .register_result_fn(
            "copy_cargo_plugin_artifact",
            ScriptCtx::copy_cargo_plugin_artifact,
        )
        .register_result_fn("dkms_install", ScriptCtx::dkms_install)
        .register_result_fn("dkms_install", ScriptCtx::dkms_install_tarball)
        .register_fn("info", info)
        .register_fn("error", error)
        .register_fn("name_to_lib", name2lib)
        .register_fn("name_to_lib_with_arch", name2lib_with_arch)
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
