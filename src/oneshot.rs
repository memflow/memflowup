use crate::database::Branch;
use crate::package::*;
use crate::Result;

pub fn install(
    to_install: &[String],
    system_wide: bool,
    dev: bool,
    reinstall: bool,
    load_opts: PackageLoadOpts,
) -> Result<()> {
    let packages = load_packages(system_wide, load_opts)?;

    let branch: Branch = dev.into();

    println!("using {} channel", branch.filename());

    let opts = PackageOpts {
        reinstall,
        system_wide,
        ..Default::default()
    };

    for p in packages
        .into_iter()
        .filter(|p| p.is_in_channel(branch))
        .filter(Package::supported_by_platform)
        .filter(|p| to_install.contains(&p.name))
    {
        println!("Installing {}", p.name);
        p.install_source(branch, &opts)?;
    }

    Ok(())
}
