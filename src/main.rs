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
use package::PackageLoadOpts;

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
        Some(("build", matches)) => build_mode::build(
            matches.value_of("name").unwrap(),
            matches.value_of("path").unwrap(),
            matches.value_of("script"),
            matches.value_of("type").unwrap(),
            matches.occurrences_of("unsafe") > 0,
            matches.occurrences_of("sys") > 0,
            matches.occurrences_of("nocopy") > 0,
        ),
        Some(("install", matches)) => oneshot::install(
            &matches.values_of_lossy("packages").unwrap(),
            matches.occurrences_of("system") > 0,
            matches.occurrences_of("dev") > 0,
            matches.occurrences_of("reinstall") > 0,
            matches.occurrences_of("from-source") > 0,
            parse_load_opts(matches),
        ),
        Some(("list", matches)) => package::list_all(
            matches.occurrences_of("system") > 0,
            parse_load_opts(matches),
        ),
        Some(("update", matches)) => package::update(
            matches.occurrences_of("system") > 0,
            matches.occurrences_of("dev") > 0,
            parse_load_opts(matches),
        ),
        Some(("interactive", matches)) => setup_mode::setup_mode(parse_load_opts(matches)),
        _ => Ok(()),
    }
}

fn add_package_opts<'a, 'b>(app: Command) -> Command {
    app.arg(Arg::new("ignore-user-index").long("ignore-user-index"))
        .arg(Arg::new("ignore-upstream-index").long("ignore-upstream-index"))
        .arg(Arg::new("ignore-builtin-index").long("ignore-builtin-index"))
}

fn parse_args() -> ArgMatches {
    Command::new("memflowup")
        .arg_required_else_help(true)
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::new("verbose").short('v').multiple_occurrences(true))
        .subcommand(
            add_package_opts(
                Command::new("install")
                    .about("Single-shot install")
                    .visible_aliases(&["install", "i"]),
            )
            .arg(Arg::new("system").long("system").short('s'))
            .arg(Arg::new("dev").long("dev").short('d'))
            .arg(Arg::new("reinstall").long("reinstall").short('r'))
            .arg(Arg::new("from-source").long("from-source").short('S'))
            .arg(Arg::new("packages").required(true).multiple_values(true)),
        )
        .subcommand(
            add_package_opts(
                Command::new("list")
                    .about("Lists all installed packages")
                    .visible_aliases(&["list", "l"]),
            )
            .arg(Arg::new("system").long("system").short('s')),
        )
        .subcommand(
            add_package_opts(
                Command::new("update")
                    .about("Updates all installed packages")
                    .visible_aliases(&["update", "u"]),
            )
            .arg(Arg::new("system").long("system").short('s'))
            .arg(Arg::new("dev").long("dev").short('d')),
        )
        .subcommand(add_package_opts(
            Command::new("interactive")
                .about("Interactive install")
                .visible_aliases(&["interactive", "I"]),
        ))
        .subcommand(
            Command::new("build")
                .about("Build and install a local project")
                .visible_aliases(&["build", "b"])
                .arg(
                    Arg::new("name")
                        .long("name")
                        .short('n')
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::new("path")
                        .long("path")
                        .short('p')
                        .takes_value(true)
                        .default_value("."),
                )
                .arg(
                    Arg::new("script")
                        .long("script")
                        .short('s')
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::new("type")
                        .long("type")
                        .short('t')
                        .takes_value(true)
                        .default_value("core_plugin"),
                )
                .arg(
                    Arg::new("unsafe")
                        .long("unsafe")
                        .short('u')
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::new("sys")
                        .long("sys")
                        .short('g')
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::new("nocopy")
                        .long("nocopy")
                        .takes_value(false)
                        .required(false),
                ),
        )
        .get_matches()
}

fn parse_load_opts(matches: &ArgMatches) -> PackageLoadOpts {
    let ignore_user = matches.occurrences_of("ignore-user-index") > 0;
    let ignore_upstream = matches.occurrences_of("ignore-upstream-index") > 0;
    let ignore_builtin = matches.occurrences_of("ignore-builtin-index") > 0;

    PackageLoadOpts::new(ignore_user, ignore_upstream, ignore_builtin)
}
