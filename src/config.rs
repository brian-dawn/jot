use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub journal_path: PathBuf,
}

/// Default journal path for new users.
fn default_journal_path() -> Option<PathBuf> {
    let mut base = dirs::home_dir()?;
    base.push(".jot");

    Some(base)
}
/// Construct the jot config path.
fn config_path() -> Option<PathBuf> {
    let mut base = dirs::home_dir()?;
    base.push(".config");
    base.push("jot");
    base.push("config.toml");

    Some(base)
}

/// Loads the config, if it doesn't exist we will create it and return the default.
pub fn load_config() -> Result<Config> {
    let default_journal_path =
        default_journal_path().context("failed to get default journal path")?;

    let default_config = Config {
        journal_path: default_journal_path.clone(),
    };

    let path = config_path().context("failed to get config path")?;

    // Lets create the default config if it doesn't exist.
    if !path.exists() {
        println!("creating {}", path.to_str().unwrap());
        std::fs::create_dir_all(
            path.parent()
                .context("failed to get parent to config path")?,
        )?;
        let toml = toml::to_string(&default_config)?;
        let mut file = File::create(&path)?;
        file.write_all(toml.as_bytes())?;

        if !default_journal_path.exists() {
            println!(
                "creating a default journal for you at {}, feel free to change this in {}",
                default_journal_path.to_str().unwrap(),
                path.to_str().unwrap()
            );
            std::fs::create_dir(&default_journal_path)
                .expect("failed to create default journal file");
        }
        Ok(default_config)
    } else {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config: Config = toml::from_str(&contents).expect("failed to parse config");
        // Make sure the journal exists.
        if !config.journal_path.exists() {
            println!(
                "your journal path specified in the config does not appear to exist {}",
                config.journal_path.to_str().unwrap()
            );
            std::process::exit(1)
        }

        Ok(config)
    }
}
