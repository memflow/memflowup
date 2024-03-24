use crate::error::Error;
use crate::package::*;

use crate::Result;

pub fn build(
    name: &str,
    path: &str,
    script: Option<&str>,
    ty: &str,
    unsafe_commands: bool,
    system_wide: bool,
    nocopy: bool,
) -> Result<()> {
    let ty = match ty {
        "core_plugin" | "core" => PackageType::CorePlugin,
        "utility" | "util" => PackageType::Utility,
        "library" | "lib" => PackageType::Library,
        "daemon_plugin" | "daemon" => PackageType::DaemonPlugin,
        _ => return Err(Error::NotImplemented("Invalid type".into())),
    };

    let package = Package {
        name: name.into(),
        repo_root_url: path.into(),
        install_script_path: script.map(<_>::into),
        unsafe_commands,
        ty,
        dev_branch: None,
        dev_binary_tag: None,
        stable_branch: None,
        stable_binary_tag: None,
        platforms: None,
    };

    let opts = PackageOpts {
        is_local: true,
        nocopy,
        system_wide,
        reinstall: true,
        from_source: true,
    };

    package.install_local(&opts)?;

    Ok(())
}
