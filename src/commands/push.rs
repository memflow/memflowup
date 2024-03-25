use clap::{Arg, ArgAction, ArgMatches, Command};
use memflow_registry_client::shared::{SignatureGenerator, MEMFLOW_DEFAULT_REGISTRY};

use crate::error::Result;

#[inline]
pub fn metadata() -> Command {
    Command::new("push").args([
        Arg::new("file_name")
            .help("the file to upload")
            .required(true)
            .action(ArgAction::Set),
        Arg::new("registry")
            .short('r')
            .long("registry")
            .help("pushes the plugin to a custom registry")
            .default_value(MEMFLOW_DEFAULT_REGISTRY)
            .action(ArgAction::Set),
        Arg::new("token")
            .short('t')
            .long("token")
            .help("the bearer token used in the upload request")
            .action(ArgAction::Set),
        Arg::new("priv-key")
            .short('p')
            .long("priv-key")
            .help("the private key used to sign the binary")
            .required(true)
            .action(ArgAction::Set),
    ])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let file_name = matches.get_one::<String>("file_name").unwrap();
    let registry = matches.get_one::<String>("registry").unwrap();
    let token = matches.get_one::<String>("token");
    let priv_key = matches.get_one::<String>("priv-key").unwrap();

    // TODO: upload progress

    let mut generator = SignatureGenerator::new(priv_key)?;
    match memflow_registry_client::upload(
        Some(registry),
        token.map(String::as_str),
        file_name,
        &mut generator,
    )
    .await
    {
        Ok(_) => {
            println!(
                "{} Uploaded plugin {:?}",
                console::style("[=]").bold().dim().green(),
                file_name
            );
        }
        Err(msg) => {
            println!(
                "{} Unable to upload plugin {:?}: {}",
                console::style("[X]").bold().dim().red(),
                file_name,
                msg
            );
        }
    }

    Ok(())
}
