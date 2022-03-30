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

    for install_name in to_install.iter() {
        let target = packages
            .iter()
            .filter(|p| p.supports_install_mode(branch, from_source))
            .filter(|&p| Package::supported_by_platform(p))
            .find(|p| p.name == install_name.as_ref());

        let mut failure = false;
        match target {
            Some(target) => {
                println!("Installing {}:", target.name);
                target.install(branch, &opts)?;
                println!();
            }
            None => {
                println!(
                    "Package '{}' was not found in '{}' channel.",
                    install_name,
                    branch.filename()
                );
                failure = true;
            }
        }

        if failure {
            println!("Some packages failed to install, try 'memflowup list' to see all available packages.");
        }
    }

    Ok(())
}
