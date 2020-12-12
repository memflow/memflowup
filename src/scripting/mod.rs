use crate::util;

use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use log::{error, info};

use rhai::{Engine, EvalAltResult, OptimizationLevel, RegisterFn, Scope, INT};

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

/// Returns the memflow connector user installation directory
fn connector_user_dir() -> String {
    dirs::home_dir()
        .unwrap()
        .join(".local")
        .join("lib")
        .join("memflow")
        .to_string_lossy()
        .to_string()
}

/// Returns the memflow connector system installation directory
fn connector_system_dir() -> String {
    dirs::home_dir()
        .unwrap()
        .join(".local")
        .join("lib")
        .join("memflow")
        .to_string_lossy()
        .to_string()
}

/// Downloads the given url to the destination file
fn download_file(url: &str, file: &str) -> bool {
    info!("download file from '{}' to '{}'", url.clone(), file.clone());
    let bytes = match util::download_file(url) {
        Ok(b) => b,
        Err(err) => {
            error!("{}", err);
            return false;
        }
    };

    match fs::write(file, bytes) {
        Ok(()) => true,
        Err(err) => {
            error!("{}", err);
            false
        }
    }
}

// TODO:
// - function that allows downloading an entire git repository
// - git submodule manipulation
// - cargo command
// - make command
fn download_zip(url: &str, folder: &str) -> bool {
    info!(
        "download zip file from '{}' to '{}'",
        url.clone(),
        folder.clone()
    );
    let bytes = match util::download_file(url) {
        Ok(b) => b,
        Err(err) => {
            error!("{}", err);
            return false;
        }
    };

    let zip_cursor = std::io::Cursor::new(&bytes[..]);
    let mut zip_archive = match ZipArchive::new(zip_cursor) {
        Ok(archive) => archive,
        Err(err) => {
            error!("{:?}", err);
            return false;
        }
    };

    for i in 0..zip_archive.len() {
        if let Ok(mut file) = zip_archive.by_index(i) {
            let outpath = PathBuf::from(folder).join(file.sanitized_name());
            if file.is_dir() {
                fs::create_dir_all(outpath).ok();
            } else {
                info!("extracing file {:?}", outpath);
                let mut outfile = File::create(&outpath).expect("unable to write output file");
                io::copy(&mut file, &mut outfile).expect("unable to write output file");
            }

        }
    }

    true
}

pub fn execute<P: AsRef<Path>>(path: P) -> () {
    let mut engine = Engine::new();

    engine
        .register_fn("info", info)
        .register_fn("error", error)
        .register_fn("tmp_dir", tmp_dir)
        .register_fn("tmp_file", tmp_file)
        .register_fn("tmp_folder", tmp_file)
        .register_fn("connector_user_dir", connector_user_dir)
        .register_fn("connector_system_dir", connector_system_dir)
        .register_fn("download_file", download_file)
        .register_fn("download_zip", download_zip);

    let mut scope = Scope::new();
    let ast = engine
        .compile_file_with_scope(&mut scope, path.as_ref().into())
        .unwrap();
    let _: () = engine.eval_ast_with_scope(&mut scope, &ast).unwrap();

    let result: bool = engine
        .call_fn(&mut scope, &ast, "build_from_git", ())
        .unwrap();

    println!("result={}", result);

    //Ok(())
}
