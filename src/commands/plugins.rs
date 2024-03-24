use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::{error::Result, registry};

#[inline]
pub fn metadata() -> Command {
    Command::new("plugins")
        .subcommand_required(true)
        .subcommands([Command::new("ls").arg(Arg::new("plugin_name").action(ArgAction::Set))])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    // TODO: list local + remote plugins
    // TODO: allow changing registry
    match matches.subcommand() {
        Some(("ls", matches)) => {
            if let Some(plugin_name) = matches.get_one::<String>("plugin_name") {
                // list versions of a specific plugin
                let plugins = registry::plugin_versions(None, plugin_name).await?;

                // TODO: dedup versions

                println!("{0: <16} {1: <16} {2: <16} {3}", "NAME", "VERSION", "DIGEST", "UPLOADED");
                for plugin in plugins.iter() {
                    println!("{0: <16} {1: <16} {2: <16} {3}", plugin_name, plugin.descriptor.version, &plugin.digest[..7], plugin.created_at);
                }
                //println!("{0: <16} {1: <10}", "TOTAL", plugins.len());

            } else {
                // list all plugins
                let plugins = registry::plugins(None).await?;

                println!("{0: <16} {1}", "NAME", "DESCRIPTION");
                for plugin in plugins.iter() {
                    println!("{0: <16} {1}", plugin.name, plugin.description);
                }
                //println!("{0: <16} {1: <10}", "TOTAL", plugins.len());
            }

            Ok(())
        }
        _ => {
            unreachable!()
        }
    }
}
