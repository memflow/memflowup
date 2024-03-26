use std::path::PathBuf;

use clap::Command;
use memflow_registry_client::shared::PluginVariant;

pub mod config;
pub mod plugins;
pub mod pull;
pub mod push;
pub mod registry;

#[inline]
pub fn metadata() -> Vec<Command> {
    vec![
        config::metadata(),
        plugins::metadata(),
        pull::metadata(),
        push::metadata(),
        registry::metadata(),
    ]
}

// TODO: move this to utils
/// Returns the path in which memflow plugins are stored.
///
/// On unix this is returns ~/.local/lib/memflow
/// On windows this returns C:\Users\[Username]\Documents\memflow
pub(crate) fn plugins_path() -> PathBuf {
    if cfg!(unix) {
        dirs::home_dir()
            .unwrap()
            .join(".local")
            .join("lib")
            .join("memflow")
    } else {
        dirs::document_dir().unwrap().join("memflow")
    }
}

// TODO: move this to utils
/// Returns the path in which memflowup config is stored.
pub(crate) fn config_path() -> PathBuf {
    if cfg!(unix) {
        dirs::home_dir().unwrap().join(".config").join("memflowup")
    } else {
        dirs::document_dir().unwrap()
    }
}

/// Returns the path that points to the memflowup config.
#[inline]
pub(crate) fn config_file_path() -> PathBuf {
    config_path().join("config.json")
}

/// Constructs the filename of this plugin for the current os.
///
/// On unix this returns libmemflow_[name]_[digest].so/.dylib
/// On windows this returns memflow_[name]_[digest].dll
pub(crate) fn plugin_file_name(variant: &PluginVariant) -> PathBuf {
    let mut file_name = plugins_path();

    // prepend the library name and append the file digest
    if cfg!(unix) {
        file_name.push(&format!(
            "libmemflow_{}_{}",
            variant.descriptor.name,
            &variant.digest[..7]
        ))
    } else {
        file_name.push(&format!(
            "memflow_{}_{}",
            variant.descriptor.name,
            &variant.digest[..7]
        ))
    }

    // append appropriate file extension
    file_name.set_extension(plugin_extension());

    file_name
}

/// Returns the plugin extension appropriate for the current os
pub(crate) fn plugin_extension() -> &'static str {
    #[cfg(target_os = "windows")]
    return "dll";
    #[cfg(target_os = "linux")]
    return "so";
    #[cfg(target_os = "macos")]
    return "dylib";
}
