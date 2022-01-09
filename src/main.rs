mod build_mode;
mod database;
mod github_api;
mod oneshot;
mod package;
mod scripting;
mod setup_mode;
mod util;

use clap::*;
use log::Level;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let matches = parse_args();

    // set log level
    let log_level = match matches.occurrences_of("verbose") {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 => Level::Trace,
        _ => Level::Trace,
    };
    simplelog::TermLogger::init(
        log_level.to_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    match matches.subcommand() {
        ("build", Some(matches)) => build_mode::build(
            matches.value_of("name").unwrap(),
            matches.value_of("path").unwrap(),
            matches.value_of("script"),
            matches.value_of("type").unwrap(),
            matches.occurrences_of("unsafe") > 0,
            matches.occurrences_of("sys") > 0,
            matches.occurrences_of("nocopy") > 0,
        ),
        ("install", Some(matches)) => oneshot::install(
            &matches.values_of_lossy("packages").unwrap(),
            matches.occurrences_of("system") > 0,
            matches.occurrences_of("dev") > 0,
            matches.occurrences_of("reinstall") > 0,
        ),
        ("list", Some(matches)) => package::list(
            matches.occurrences_of("system") > 0,
            (matches.occurrences_of("dev") > 0).into(),
        ),
        ("update", Some(matches)) => package::update(
            matches.occurrences_of("system") > 0,
            (matches.occurrences_of("dev") > 0).into(),
        ),
        ("interactive", Some(_)) => setup_mode::setup_mode(),
        _ => Ok(()),
    }
}

fn parse_args() -> ArgMatches<'static> {
    App::new("memflowup")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::with_name("verbose").short("v").multiple(true))
        .subcommand(
            SubCommand::with_name("install")
                .about("Single-shot install")
                .visible_aliases(&["install", "i"])
                .arg(Arg::with_name("system").long("system").short("s"))
                .arg(Arg::with_name("dev").long("dev").short("d"))
                .arg(Arg::with_name("reinstall").long("reinstall").short("r"))
                .arg(Arg::with_name("packages").required(true).multiple(true)),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists all installed packages")
                .visible_aliases(&["list", "l"])
                .arg(Arg::with_name("system").long("system").short("s"))
                .arg(Arg::with_name("dev").long("dev").short("d")),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Updates all installed packages")
                .visible_aliases(&["update", "u"])
                .arg(Arg::with_name("system").long("system").short("s"))
                .arg(Arg::with_name("dev").long("dev").short("d")),
        )
        .subcommand(
            SubCommand::with_name("interactive")
                .about("Interactive install")
                .visible_aliases(&["interactive", "I"]),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Build and install a local project")
                .visible_aliases(&["build", "b"])
                .arg(
                    Arg::with_name("name")
                        .long("name")
                        .short("n")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("path")
                        .long("path")
                        .short("p")
                        .takes_value(true)
                        .default_value("."),
                )
                .arg(
                    Arg::with_name("script")
                        .long("script")
                        .short("s")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("type")
                        .long("type")
                        .short("t")
                        .takes_value(true)
                        .default_value("core_plugin"),
                )
                .arg(
                    Arg::with_name("unsafe")
                        .long("unsafe")
                        .short("u")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("sys")
                        .long("sys")
                        .short("g")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("nocopy")
                        .long("nocopy")
                        .takes_value(false)
                        .required(false),
                ),
        )
        .get_matches()
}
