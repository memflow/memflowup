use serde::*;

use log::*;

use std::collections::HashMap;

use std::path::PathBuf;

use crate::util;

#[derive(Clone, Serialize, Deserialize)]
pub struct DatabaseEntry {
    pub ty: EntryType,
    pub artifacts: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum EntryType {
    /// Git commit hash
    GitSource(String),
    /// Release tag
    Binary(String),
}

fn dev_string(dev_branch: bool) -> &'static str {
    if dev_branch {
        "dev"
    } else {
        "stable"
    }
}

fn db_path(dev_branch: bool, system_db: bool) -> PathBuf {
    let db_name = format!("db2.{}.json", dev_string(dev_branch));

    if system_db {
        #[cfg(not(windows))]
        {
            format!("/etc/memflowup/{}", db_name).into()
        }
        #[cfg(windows)]
        // TODO: pick a better path
        {
            format!("C:\\memflowup\\{}", db_name).into()
        }
    } else {
        let mut path = dirs::config_dir().unwrap();
        path.push("memflowup");
        path.push(db_name);
        path
    }
}

pub fn load_database(
    dev_branch: bool,
    system_db: bool,
) -> Result<HashMap<String, DatabaseEntry>, serde_json::Error> {
    let path = db_path(dev_branch, system_db);

    if let Ok(file) = std::fs::File::open(path) {
        serde_json::from_reader(file)
    } else {
        info!("Database does not exist, returning new!");
        Ok(Default::default())
    }
}

pub fn commit_entry(
    name: &str,
    entry: DatabaseEntry,
    dev_branch: bool,
    system_db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = db_path(dev_branch, system_db);

    let mut dir = path.clone();
    dir.pop();

    std::fs::create_dir_all(dir)?;

    let mut db = load_database(dev_branch, system_db)?;

    // TODO: cleanup any artifacts that no longer exist?

    db.insert(name.to_string(), entry);

    match std::fs::File::create(&path) {
        Ok(file) => serde_json::to_writer_pretty(file, &db).map_err(Into::into),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied && system_db {
                let dir = util::make_temp_dir("memflowup_build", &["db.json"])?;
                let tmp_path = dir.join("db.json");
                let file = std::fs::File::create(&tmp_path)?;
                serde_json::to_writer_pretty(file, &db)?;
                util::copy_file(&tmp_path, &path, true).map_err(Into::into)
            } else {
                Err(err.into())
            }
        }
    }
}
