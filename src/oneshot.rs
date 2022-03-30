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

    let opts = PackageOpts {
        reinstall,
        system_wide,
        from_source,
        ..Default::default()
    };

    for install_name in to_install.iter() {
        let target = packages
            .iter()
            .filter(|p| p.supported_by_platform())
            .find(|p| install_name == p.name.as_str());

        let mut not_found = false;
        match target {
            Some(target) if target.supports_install_mode(branch, from_source) => {
                println!("Installing {}:", target.name);
                target.install(branch, &opts)?;
                println!();
            }
            Some(target) => {
                println!(
                    "Package '{}' was not found in '{}' channel via '{}' installation.",
                    install_name,
                    branch.filename(),
                    if from_source { "git" } else { "binary" }
                );
                println!(
                    "'{}' is only available via the following channels: {}",
                    install_name,
                    target.available_modes().join(", ")
                );
                println!("Use 'memflowup install --help' to see additional flags for choosing a channel.");
            }
            None => {
                println!("Package '{}' was not found.", install_name,);
                not_found = true;
            }
        }

        if not_found {
            println!(
                "Some packages failed to install, try 'memflowup {}' to see all available packages.",
                if system_wide { "list -s" } else { "list" }
            );
        }
    }

    Ok(())
}
