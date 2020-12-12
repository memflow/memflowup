mod scripting;
mod setup_mode;
mod util;

use log::Level;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(Level::Info.to_level_filter())
        .init()
        .unwrap();

    // TODO: check if we have cmdline args and only run setup mode in specific cases
    setup_mode::setup_mode();
}
