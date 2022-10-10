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
    let log_level = match matches.get_count("verbose") {
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
            matches.get_one::<String>("name").unwrap(),
            matches.get_one::<String>("path").unwrap(),
            matches.get_one::<String>("script").map(String::as_str),
            matches.get_one::<String>("type").unwrap(),
            matches.get_flag("unsafe"),
            matches.get_flag("sys"),
            matches.get_flag("nocopy"),
        ),
        Some(("install", matches)) => oneshot::install(
            &matches
                .get_many("packages")
                .unwrap()
                .cloned()
                .collect::<Vec<_>>(),
            matches.get_flag("system"),
            matches.get_flag("dev"),
            matches.get_flag("reinstall"),
            matches.get_flag("from-source"),
            parse_load_opts(matches),
        ),
        Some(("list", matches)) => {
            package::list_all(matches.get_flag("system"), parse_load_opts(matches))
        }
        Some(("update", matches)) => package::update(
            matches.get_flag("system"),
            matches.get_flag("dev"),
            parse_load_opts(matches),
        ),
        Some(("interactive", matches)) => setup_mode::setup_mode(parse_load_opts(matches)),
        _ => Ok(()),
    }
}

fn add_package_opts<'a, 'b>(app: Command) -> Command {
    app.arg(
        Arg::new("ignore-user-index")
            .long("ignore-user-index")
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new("ignore-upstream-index")
            .long("ignore-upstream-index")
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new("ignore-builtin-index")
            .long("ignore-builtin-index")
            .action(ArgAction::SetTrue),
    )
}

fn parse_args() -> ArgMatches {
    Command::new("memflowup")
        .arg_required_else_help(true)
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
        .subcommand(
            add_package_opts(
                Command::new("install")
                    .about("Single-shot install")
                    .visible_alias("i"),
            )
            .arg(
                Arg::new("system")
                    .long("system")
                    .short('s')
                    .help("Enables system-wide installation for all users")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("dev")
                    .long("dev")
                    .short('d')
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("reinstall")
                    .long("reinstall")
                    .short('r')
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("from-source")
                    .long("from-source")
                    .short('S')
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("packages")
                    .required(true)
                    .action(ArgAction::Append),
            ),
        )
        .subcommand(
            add_package_opts(
                Command::new("list")
                    .about("Lists all installed packages")
                    .visible_alias("l"),
            )
            .arg(
                Arg::new("system")
                    .long("system")
                    .short('s')
                    .help("Enables system-wide installation for all users")
                    .action(ArgAction::SetTrue),
            ),
        )
        .subcommand(
            add_package_opts(
                Command::new("update")
                    .about("Updates all installed packages")
                    .visible_alias("u"),
            )
            .arg(
                Arg::new("system")
                    .long("system")
                    .short('s')
                    .help("Enables system-wide installation for all users")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("dev")
                    .long("dev")
                    .short('d')
                    .action(ArgAction::SetTrue),
            ),
        )
        .subcommand(add_package_opts(
            Command::new("interactive").about("Interactive install"),
        ))
        .subcommand(
            Command::new("build")
                .about("Build and install a local project")
                .visible_alias("b")
                .arg(Arg::new("name").long("name").short('n').required(true))
                .arg(Arg::new("path").long("path").short('p').default_value("."))
                .arg(Arg::new("script").long("script").short('s'))
                .arg(
                    Arg::new("type")
                        .long("type")
                        .short('t')
                        .default_value("core_plugin"),
                )
                .arg(
                    Arg::new("unsafe")
                        .long("unsafe")
                        .short('u')
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("sys")
                        .long("sys")
                        .short('g')
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("nocopy").long("nocopy").action(ArgAction::SetTrue)),
        )
        .get_matches()
}

fn parse_load_opts(matches: &ArgMatches) -> PackageLoadOpts {
    let ignore_user = matches.get_flag("ignore-user-index");
    let ignore_upstream = matches.get_flag("ignore-upstream-index");
    let ignore_builtin = matches.get_flag("ignore-builtin-index");

    PackageLoadOpts::new(ignore_user, ignore_upstream, ignore_builtin)
}
