use clap::Command;

pub mod plugins;
pub mod pull;
pub mod push;

#[inline]
pub fn metadata() -> Vec<Command> {
    vec![plugins::metadata(), pull::metadata(), push::metadata()]
}
