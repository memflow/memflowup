use serde::*;

use crate::scripting;

const DEFAULT_INDEX: &str = include_str!("../index.json");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Package {
    pub name: String,
    pub ty: PackageType,
    pub repo_root_url: String,
    pub stable_branch: Option<String>,
    pub dev_branch: Option<String>,
    #[serde(default)]
    pub unsafe_commands: bool,
    pub install_script_path: Option<String>,
}

impl Package {
    pub fn install_source(&self, dev_branch: bool) {
        scripting::execute_installer(self, dev_branch, "build_from_source").unwrap();
    }

    pub fn is_in_channel(&self, dev_branch: bool) -> bool {
        self.branch(dev_branch).is_some()
    }

    pub fn branch(&self, dev_branch: bool) -> Option<&str> {
        if dev_branch {
            self.dev_branch.as_deref()
        } else {
            self.stable_branch.as_deref()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PackageType {
    CorePlugin,
    Utility,
    Library,
    DaemonPlugin,
}

pub fn load_packages() -> Vec<Package> {
    serde_json::from_str(DEFAULT_INDEX).unwrap()
}
