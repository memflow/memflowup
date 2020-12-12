use crate::util;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use log::{error, info};

use rhai::{Engine, EvalAltResult, OptimizationLevel, RegisterFn, Scope, INT};

fn info(s: &str) -> () {
    info!("{}", s);
}

fn error(s: &str) -> () {
    error!("{}", s);
}

fn tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

fn tmp_file(name: &str) -> String {
    env::temp_dir().join(name).to_string_lossy().to_string()
}

fn download(url: &str, file: &str) -> bool {
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

pub fn execute<P: AsRef<Path>>(path: P) -> () {
    let mut engine = Engine::new();

    engine
        .register_fn("info", info)
        .register_fn("error", error)
        .register_fn("tmp_dir", tmp_dir)
        .register_fn("tmp_file", tmp_file)
        .register_fn("download", download);

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
