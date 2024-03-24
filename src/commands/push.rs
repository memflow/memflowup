use clap::{ArgMatches, Command};

use crate::error::Result;

#[inline]
pub fn metadata() -> Command {
    Command::new("push")
}

pub async fn command(matches: &ArgMatches) -> Result<()> {
    // TODO: - sign plugin (ask user for signature file if necessary)
    // TODO: - upload plugin with signature
    // TODO: - ask user for token if it wasnt found in environment variables

    Ok(())
}
