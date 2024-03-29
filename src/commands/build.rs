//! Clap subcommand to configure memflowup

use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::{Arg, ArgAction, ArgMatches};
use memflow_registry_client::shared::PluginVariant;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::{
    ensure_rust,
    error::{Error, Result},
    github_api,
    util::{self, create_temp_dir},
};

#[inline]
pub fn metadata() -> clap::Command {
    clap::Command::new("build").args([
        Arg::new("repository_or_path").help("url to the git repository to pull from (e.g. https://github.com/memflow/memflow-coredump) or local path").required(true),
        Arg::new("path")
            .long("path")
            .short('p')
            .help("file system path to local plugin source to install")
            .action(ArgAction::SetTrue),
        Arg::new("branch").long("branch").help("checks out the git repository at this specific branch").action(ArgAction::Set),
        Arg::new("tag").long("tag").help("checks out the git repository at this specific tag").action(ArgAction::Set),
        Arg::new("all-features")
            .long("all-features")
            .help("builds the plugin with the --all-features flag")
            .action(ArgAction::SetTrue),
    ])
}

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    // rust / cargo is required for source builds
    ensure_rust::ensure_rust().await?;

    let repository_or_path = matches.get_one::<String>("repository_or_path").unwrap();
    let path = matches.get_flag("path");

    let all_features = matches.get_flag("all-features");

    if !path {
        // download and install from a repository
        // TODO: support non-github repos
        // TODO: print proper not found error instead of a random error
        let commit = if let Some(tag) = matches.get_one::<String>("tag") {
            let tag = github_api::tag(repository_or_path, tag).await?;
            tag.commit.sha
        } else {
            let branch = matches
                .get_one::<String>("branch")
                .map(String::as_str)
                .unwrap_or_else(|| "main");
            let branch = github_api::branch(repository_or_path, branch).await?;
            branch.commit.sha
        };

        // create temporary directory (will be dropped when this code path exits)
        let temp_dir = create_temp_dir("memflowup_build", &commit).await?;

        // run compilation and installation
        let source_path =
            download_from_repository(repository_or_path, &commit, temp_dir.as_path()).await?;
        let artifacts = build_artifacts_from_source(&source_path, all_features).await?;
        for artifact in artifacts.iter() {
            install_artifact(artifact).await.ok();
        }
    } else {
        // install from local path
        let path = Path::new(repository_or_path);
        if !path.exists() || !path.is_dir() {
            println!(
                "{} Path does not exist or is not a directory.",
                console::style("[-]").bold().dim(),
            );
            return Err(Error::NotFound(
                "path does not exist or is not a directory".to_string(),
            ));
        }

        let artifacts = build_artifacts_from_source(path, all_features).await?;
        for artifact in artifacts.iter() {
            install_artifact(artifact).await.ok();
        }
    }

    Ok(())
}

/// Downloads the repository and returns the temporary path in which the contents was extracted.
async fn download_from_repository(
    repository: &str,
    commit: &str,
    temp_dir_path: &Path,
) -> Result<PathBuf> {
    // query file and download to memory
    println!(
        "{} Downloading plugin source from {} with commit {}",
        console::style("[-]").bold().dim(),
        repository,
        commit
    );
    let response = github_api::download_code_for_commit(repository, commit).await?;
    let buffer = util::read_response_with_progress(response).await?;

    // create temporary build directory
    // TODO: replace https://docs.rs/tempfile/latest/tempfile/fn.tempdir.html
    let build_hash = sha256::digest(buffer.as_ref());
    let extract_path = temp_dir_path.to_path_buf().join(build_hash);

    // unpack archive
    println!("{} Unpacking source", console::style("[-]").bold().dim(),);
    util::zip_unpack(buffer.as_ref(), &extract_path, 1)?;

    Ok(extract_path)
}

/// Builds the plugin from the given source path and returns the path of the resulting artifact.
/// For workspace repos this can return a list of artifacts.
async fn build_artifacts_from_source(
    source_path: &Path,
    all_features: bool,
) -> Result<Vec<PathBuf>> {
    // build plugin
    println!(
        "{} Building plugin in: {:?}",
        console::style("[-]").bold().dim(),
        source_path,
    );
    if all_features {
        let _ = util::cargo("build --release --all-features", source_path)?;
    } else {
        let _ = util::cargo("build --release", source_path)?;
    }

    // try to find a valid artifact in the build folder
    let artifact_path = source_path.to_path_buf().join("target").join("release");
    let paths = std::fs::read_dir(artifact_path)?;
    let mut artifacts = Vec::new();
    for path in paths.filter_map(|p| p.ok()) {
        if path.path().is_file() {
            if let Some(extension) = path.path().extension() {
                if extension.to_str().unwrap_or_default() == util::plugin_extension() {
                    println!(
                        "{} Plugin artifact successfully built: {:?}",
                        console::style("[=]").bold().dim().green(),
                        path.path()
                    );
                    artifacts.push(path.path());
                }
            }
        }
    }

    // extract the artifact file name
    if !artifacts.is_empty() {
        Ok(artifacts)
    } else {
        println!(
                    "{} No valid build artifact with the `{}` file extension found. Are you sure this is a dylib project?",
                    console::style("[-]").bold().dim(),
                    util::plugin_extension(),
                );
        Err(Error::NotFound(
            "no supported build artifact found.".to_string(),
        ))
    }
}

async fn install_artifact(artifact_path: &Path) -> Result<()> {
    // parse the plugins descriptor
    let artifact_content = tokio::fs::read(artifact_path).await?;
    let descriptors =
        memflow_registry_client::shared::plugin_analyzer::parse_descriptors(&artifact_content)?;

    // construct variant of this plugin, for now we only use the first descriptor found
    // TODO: support multiple descriptors
    // TODO: currently we do not ensure that digest is identical each time we build it.
    // TODO: We should ensure the build timestamps match to have truly reproducible builds
    let variant = match descriptors.first() {
        Some(descriptor) => PluginVariant {
            digest: sha256::digest(&artifact_content),
            signature: String::new(),
            created_at: Utc::now().naive_utc(),
            descriptor: descriptor.clone(),
        },
        None => {
            println!(
                    "{} PluginDescriptor not found in artifact {:?}. Are you sure this is a memflow plugin project?",
                    console::style("[-]").bold().dim(),
                    artifact_path
                );
            return Err(Error::NotFound(
                "no supported build artifact found.".to_string(),
            ));
        }
    };

    // construct destination file_name in memflowup registry
    let file_name = util::plugin_file_name(&variant);
    if file_name.exists() {
        println!(
            "{} Plugin already exists, overwriting.",
            console::style("[-]").bold().dim().yellow(),
        );
    }

    // write file
    let mut file = File::create(&file_name).await?;
    file.write_all(&artifact_content).await?;
    file.flush().await?;

    println!(
        "{} Wrote plugin to: {:?}",
        console::style("[=]").bold().dim().green(),
        file_name.as_os_str(),
    );

    // store .meta file of plugin containing all relevant information
    // TODO: this does not contain all plugins in this file - allow querying that from memflow-registry as well
    let mut file_name = file_name.clone();
    file_name.set_extension("meta");
    tokio::fs::write(&file_name, serde_json::to_string_pretty(&variant)?).await?;

    println!(
        "{} Wrote plugin metadata to: {:?}",
        console::style("[=]").bold().dim().green(),
        file_name.as_os_str(),
    );
    Ok(())
}
