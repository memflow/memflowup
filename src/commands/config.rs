//! Clap subcommand to configure memflowup

use std::{cmp::Reverse, collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command};
use memflow_registry_client::shared::{PluginUri, PluginVariant};
use serde::{Deserialize, Serialize};

use crate::{
    commands::{plugin_extension, plugins_path},
    error::{Error, Result},
};

use super::config_file_path;

pub const CONFIG_KEYS: [&str; 3] = ["registry", "token", "priv_key_file"];

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Config {
    entries: HashMap<String, String>,
}

impl Config {
    #[inline]
    pub fn get(&self, key: &str) -> Result<Option<&str>> {
        if CONFIG_KEYS.iter().any(|&k| k == key) {
            Ok(self.entries.get(key).map(String::as_str))
        } else {
            Err(Error::NotFound(format!("option `{}` is invalid", key)))
        }
    }

    #[inline]
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        if CONFIG_KEYS.iter().any(|&k| k == key) {
            self.entries.insert(key.to_owned(), value.to_owned());
            Ok(())
        } else {
            Err(Error::NotFound(format!("option `{}` is invalid", key)))
        }
    }

    #[inline]
    pub fn unset(&mut self, key: &str) -> Result<()> {
        if CONFIG_KEYS.iter().any(|&k| k == key) {
            let _ = self.entries.remove(key);
            Ok(())
        } else {
            Err(Error::NotFound(format!("option `{}` is invalid", key)))
        }
    }
}

#[inline]
pub fn metadata() -> Command {
    Command::new("config")
        .subcommand_required(true)
        .subcommands([
            Command::new("get").args([Arg::new("key")]),
            Command::new("set").args([
                Arg::new("key").required(true),
                Arg::new("value").required(true),
            ]),
            Command::new("unset").args([Arg::new("key").required(true)]),
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
                for (key, value) in config.entries.iter() {
                    println!("{} = {}", key, value);
                }
            }
            Ok(())
        }
        Some(("set", matches)) => {
            let key = matches.get_one::<String>("key").unwrap();
            let value = matches.get_one::<String>("value").unwrap();

            let mut config = read_config().await?;

            if let Err(_) = config.set(key, value) {
                println!(
                    "{} Config option `{}` does not exist. Valid options are: {}",
                    console::style("[X]").bold().dim().red(),
                    key,
                    CONFIG_KEYS.join(", ")
                );
            }

            write_config(config).await
        }
        Some(("unset", matches)) => {
            let key = matches.get_one::<String>("key").unwrap();

            let mut config = read_config().await?;
            if let Err(_) = config.unset(key) {
                println!(
                    "{} Config option `{}` does not exist. Valid options are: {}",
                    console::style("[X]").bold().dim().red(),
                    key,
                    CONFIG_KEYS.join(", ")
                );
            }

            write_config(config).await
        }
        _ => unreachable!(),
    }
}

async fn ensure_config_file_path() -> Result<()> {
    if cfg!(unix) {
        // create ~/.config folder
        let path = dirs::home_dir().unwrap().join(".config");
        if !path.exists() {
            tokio::fs::create_dir(&path).await?;
        }

        // create ~/.config/memflowup folder
        let path = path.join("memflowup");
        if !path.exists() {
            tokio::fs::create_dir(&path).await?;
        }
    }

    // create file
    let path = config_file_path();
    if !path.exists() {
        tokio::fs::write(path, b"{}").await?;
    }

    Ok(())
}

async fn read_config() -> Result<Config> {
    ensure_config_file_path().await?;
    let content = tokio::fs::read_to_string(config_file_path()).await?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

async fn write_config(config: Config) -> Result<()> {
    ensure_config_file_path().await?;
    let content = serde_json::to_string(&config)?;
    Ok(tokio::fs::write(config_file_path(), content.as_bytes()).await?)
}
