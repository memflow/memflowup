//! Clap subcommand to configure memflowup

use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use memflow_registry_client::shared::{
    SignatureGenerator, SignatureVerifier, MEMFLOW_DEFAULT_REGISTRY,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    util,
};

pub const CONFIG_KEYS: [&str; 4] = ["registry", "token", "pub_key_file", "priv_key_file"];

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub registry: Option<String>,
    pub token: Option<String>,
    pub pub_key_file: Option<PathBuf>,
    pub priv_key_file: Option<PathBuf>,
}

impl Config {
    #[inline]
    pub fn get(&self, key: &str) -> Result<Option<&str>> {
        match key {
            "registry" => Ok(Some(
                self.registry.as_deref().unwrap_or(MEMFLOW_DEFAULT_REGISTRY),
            )),
            "token" => Ok(self.token.as_deref()),
            "pub_key_file" => Ok(self
                .pub_key_file
                .as_ref()
                .map(|p| p.as_os_str().to_str().unwrap())),
            "priv_key_file" => Ok(self
                .priv_key_file
                .as_ref()
                .map(|p| p.as_os_str().to_str().unwrap())),
            _ => Err(Error::NotFound(format!("option `{}` is invalid", key))),
        }
    }

    #[inline]
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "registry" => {
                self.registry = Some(value.to_owned());
                Ok(())
            }
            "token" => {
                self.token = Some(value.to_owned());
                Ok(())
            }
            "pub_key_file" => {
                let path = Path::new(value);
                if path.exists() {
                    match SignatureVerifier::new(path) {
                        Ok(_) => {
                            self.pub_key_file = path.canonicalize().ok();
                            Ok(())
                        }
                        Err(_) => Err(Error::NotFound(
                            "File is not a valid public key file".to_owned(),
                        )),
                    }
                } else {
                    Err(Error::NotFound("Key file does not exist".to_owned()))
                }
            }
            "priv_key_file" => {
                let path = Path::new(value);
                if path.exists() {
                    match SignatureGenerator::new(path) {
                        Ok(_) => {
                            self.priv_key_file = path.canonicalize().ok();
                            Ok(())
                        }
                        Err(_) => Err(Error::NotFound(
                            "File is not a valid private key file".to_owned(),
                        )),
                    }
                } else {
                    Err(Error::NotFound("Key file does not exist".to_owned()))
                }
            }
            _ => Err(Error::NotFound(format!("option `{}` is invalid", key))),
        }
    }

    #[inline]
    pub fn unset(&mut self, key: &str) -> Result<()> {
        match key {
            "registry" => {
                self.registry = None;
                Ok(())
            }
            "token" => {
                self.token = None;
                Ok(())
            }
            "pub_key_file" => {
                self.pub_key_file = None;
                Ok(())
            }
            "priv_key_file" => {
                self.priv_key_file = None;
                Ok(())
            }
            _ => Err(Error::NotFound(format!("option `{}` is invalid", key))),
        }
    }
}

#[inline]
pub fn metadata() -> Command {
    Command::new("config")
        .subcommand_required(true)
        .subcommands([
            Command::new("get").args([Arg::new("key").help("configuration entry key")]),
            Command::new("set").args([
                Arg::new("key")
                    .help("configuration entry key")
                    .required(true),
                Arg::new("value").help("configuration value to set"),
            ]),
            Command::new("unset").args([Arg::new("key")
                .help("configuration entry key")
                .required(true)]),
        ])
}

// TODO: use keychain for token/keyfile
pub async fn handle(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("get", matches)) => {
            let config = read_config().await?;
            if let Some(key) = matches.get_one::<String>("key") {
                match config.get(key) {
                    Ok(Some(value)) => println!("{}", value),
                    Err(_) => println!(
                        "{} Config option `{}` does not exist. Valid options are: {}",
                        console::style("[X]").bold().dim().red(),
                        key,
                        CONFIG_KEYS.join(", ")
                    ),
                    _ => (),
                }
            } else {
                println!("registry = \"{}\"", config.registry.unwrap_or_default());

                let token = config.token.unwrap_or_default();
                let token = if token.len() > 6 {
                    format!(
                        "{}{}",
                        &token[..4],
                        token[4..].chars().map(|_| '*').collect::<String>()
                    )
                } else {
                    token.chars().map(|_| '*').collect()
                };
                println!("token = \"{}\"", token);

                println!(
                    "pub_key_file = {:?}",
                    config.pub_key_file.unwrap_or_default()
                );
                println!(
                    "priv_key_file = {:?}",
                    config.priv_key_file.unwrap_or_default()
                );
            }
            Ok(())
        }
        Some(("set", matches)) | Some(("unset", matches)) => {
            let key = matches.get_one::<String>("key").unwrap();

            let mut config = read_config().await?;

            let result = if let Some(value) = matches.get_one::<String>("value") {
                config.set(key, value)
            } else {
                config.unset(key)
            };
            if let Err(err) = result {
                println!(
                    "{} Error setting config option `{}`: {}",
                    console::style("[X]").bold().dim().red(),
                    key,
                    err
                );
            }

            write_config(config).await
        }
        _ => unreachable!(),
    }
}

pub async fn read_config() -> Result<Config> {
    let content = tokio::fs::read_to_string(util::config_file_path()).await?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

pub async fn write_config(config: Config) -> Result<()> {
    let content = serde_json::to_string(&config)?;
    Ok(tokio::fs::write(util::config_file_path(), content.as_bytes()).await?)
}
