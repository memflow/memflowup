use crate::database::Branch;
use crate::database::{load_database, DatabaseEntry, EntryType};
use crate::package::*;
use crate::Result;

pub fn install(to_install: &[String], system_wide: bool, dev: bool) -> Result<()> {
    let packages = load_packages(system_wide)?;

    let branch: Branch = dev.into();

    println!("using {} channel", branch.filename());

    for p in packages
        .into_iter()
        .filter(|p| p.is_in_channel(branch))
        .filter(Package::supported_by_platform)
        .filter(|p| to_install.contains(&p.name))
    {
        println!("Installing {}", p.name);
        p.install_source(branch, &PackageOpts::system_wide(system_wide))?;
    }

    Ok(())
}
