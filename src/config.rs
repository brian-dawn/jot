use anyhow::{Context, Result};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub journal_path: PathBuf,
}

/// Default journal path for new users.
fn default_journal_path() -> Option<PathBuf> {
    let mut base = dirs::home_dir()?;
    base.push(".jot-journal.txt");

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

/// Fetch the path for our notified database.
/// TODO: We should probably periodically prune old entries from this.
fn notified_path() -> Option<PathBuf> {
    let mut base = dirs::data_local_dir()?;
    base.push("jot-notified");
    Some(base)
}

/// We wanna avoid spamming notifications so we keep track of notifications we've
/// already sent in this file.
pub fn load_notified() -> Result<HashSet<DateTime<Local>>> {
    // This is just a line separated file of timestamps.
    let path = notified_path().context("failed to get data path")?;
    if !path.exists() {
        println!("creating {}", path.to_str().unwrap());
        let _file = File::create(&path)?;
        return Ok(HashSet::new());
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents
        .split_whitespace()
        .filter_map(|d| {
            let parsed_date: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(&d).ok()?;
            Some(DateTime::from(parsed_date))
        })
        .collect())
}

/// Mark that a particular jot has been notified on this particular computer
/// that way we don't display it again.
pub fn mark_notified(dt: DateTime<Local>) -> Result<()> {
    let path = notified_path().context("failed to get data path")?;
    let mut file = if !path.exists() {
        println!("creating {}", path.to_str().unwrap());
        File::create(&path)?
    } else {
        OpenOptions::new().append(true).open(path)?
    };
    let date_str = dt.to_rfc3339();
    writeln!(file, "{}", date_str)?;
    Ok(())
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
            File::create(&default_journal_path).expect("failed to create default journal file");
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
