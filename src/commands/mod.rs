pub mod build;
pub mod config;
pub mod plugins;
pub mod pull;
pub mod push;
pub mod registry;

#[allow(clippy::print_literal)]
#[inline]
fn print_plugin_versions_header() {
    println!(
        "{0: <16} {1: <16} {2: <16} {3: <8} {4: <65} {5:}",
        "NAME", "VERSION", "PLUGIN_VERSION", "DIGEST", "DIGEST_LONG", "CREATED"
    );
}
