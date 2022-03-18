use crate::database::Branch;
use crate::package::*;
use crate::Result;

pub fn install(
    to_install: &[String],
    system_wide: bool,
    dev: bool,
    reinstall: bool,
    from_source: bool,
    load_opts: PackageLoadOpts,
) -> Result<()> {
    update_index(system_wide)?;
    let packages = load_packages(system_wide, load_opts)?;

    let branch: Branch = dev.into();

    println!("using {} channel", branch.filename());

    let opts = PackageOpts {
        reinstall,
        system_wide,
        from_source,
        ..Default::default()
    };

    for package in packages
        .into_iter()
        .filter(|p| p.supports_install_mode(branch, from_source))
        .filter(Package::supported_by_platform)
        .filter(|p| to_install.contains(&p.name))
    {
        println!("Installing {}:", package.name);
        package.install(branch, &opts)?;
        println!();
    }

    Ok(())
}
