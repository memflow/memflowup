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
        "{0: <16} {1: <16} {2: <12} {3: <4} {4: <8} {5: <65} {6:}",
        "NAME", "VERSION", "ARCH", "ABI", "DIGEST", "DIGEST_LONG", "CREATED"
    );
}
