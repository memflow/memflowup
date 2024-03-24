use clap::Command;

pub mod pull;
pub mod push;
pub mod registry;

#[inline]
pub fn metadata() -> Vec<Command> {
    vec![pull::metadata(), push::metadata(), registry::metadata()]
}
