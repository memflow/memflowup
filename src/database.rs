use serde::*;

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

pub fn commit_entry(name: &str, entry: DatabaseEntry, system_db: bool) {}
