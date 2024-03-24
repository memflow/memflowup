use clap::{ArgMatches, Command};

use crate::error::Result;

#[inline]
pub fn metadata() -> Command {
    Command::new("push")
}

pub async fn command(matches: &ArgMatches) -> Result<()> {
    Ok(())
}
