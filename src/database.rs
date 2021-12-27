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
    /// Local path
    LocalPath(String),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Branch {
    Dev,
    Stable,
    Local,
}

impl Branch {
    pub fn filename(&self) -> &'static str {
        match self {
            Branch::Dev => "dev",
            Branch::Stable => "stable",
            Branch::Local => "local",
        }
    }
}

impl From<bool> for Branch {
    fn from(dev: bool) -> Self {
        if dev {
            Self::Dev
        } else {
            Self::Stable
        }
    }
}

fn db_path(dev_branch: Branch, system_db: bool) -> PathBuf {
    let mut cfg_dir = util::config_dir(system_db);
    cfg_dir.push(format!("db2.{}.json", dev_branch.filename()));
    cfg_dir
}

pub fn load_database(
    dev_branch: Branch,
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
    dev_branch: Branch,
    system_db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = db_path(dev_branch, system_db);

    let mut dir = path.clone();
    dir.pop();

    util::create_dir_with_elevation(dir, system_db)?;

    let mut db = load_database(dev_branch, system_db)?;

    // TODO: cleanup any artifacts that no longer exist?

    db.insert(name.to_string(), entry);

    util::write_with_elevation(&path, system_db, |writer| {
        serde_json::to_writer_pretty(writer, &db).map_err(Into::into)
    })
}
