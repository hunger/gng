// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{Error, Result};

use std::path::PathBuf;

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn env_path(env_var: &str, default_prefix: &str, fallback: &str) -> PathBuf {
    std::env::var(env_var).map_or_else(
        |_| {
            std::env::var("HOME").map_or_else(
                |_| PathBuf::from(fallback),
                |p| PathBuf::from(p).join(default_prefix),
            )
        },
        PathBuf::from,
    )
}

fn config_path() -> PathBuf {
    env_path("XDG_CONFIG_HOME", ".config", "/etc")
}

fn cache_path() -> PathBuf {
    env_path("XDG_CACHE_HOME", ".cache", "/var/cache")
}

fn data_path() -> PathBuf {
    env_path("XDG_DATA_HOME", ".local/share", "/var/lib")
}

// Fill in default config file path if possible
fn default_config_file_path(config_file: &Option<PathBuf>) -> Option<PathBuf> {
    if config_file.is_some() {
        config_file.clone()
    } else {
        let config_file_path = config_path().join("gng/default.conf");
        if config_file_path.exists() {
            Some(config_file_path)
        } else {
            None
        }
    }
}

// ----------------------------------------------------------------------
// - Config:
// ----------------------------------------------------------------------

/// A Configuration object that can be read from a configuration file.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    /// The directory containing all data from local repositories
    pub locals_dir: PathBuf,
    /// The directory holding the packet DB
    pub packet_db_directory: PathBuf,
    /// The directory holding cached data from remote repositories
    pub remotes_dir: PathBuf,
    /// The directory holding repository definition files
    pub repository_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            locals_dir: cache_path().join("gng/remotes"),
            packet_db_directory: data_path().join("gng/packets"),
            remotes_dir: cache_path().join("gng/remotes"),
            repository_dir: config_path().join("gng/repositories"),
        }
    }
}

impl Config {
    /// Parse a config file and create a `Config` object from it.
    ///
    /// # Errors
    /// May return an `Error::Config` if opening the `config_file` or parsing the necessary values fails.
    #[tracing::instrument(level = "debug")]
    pub fn new(config_file: &Option<PathBuf>) -> Result<Self> {
        // Fill in default config file
        if let Some(config_file) = default_config_file_path(config_file) {
            let file = std::fs::File::open(&config_file)?;
            Ok(
                serde_json::from_reader(std::io::BufReader::new(file)).map_err(|_| {
                    Error::Config(format!(
                        "Could not read configuration from \"{}\".",
                        config_file.display()
                    ))
                })?,
            )
        } else {
            Ok(Self::default())
        }
    }

    /// Open the `RepositoryDB` pointed to by this `Config`
    ///
    /// # Errors
    /// May return a `Error::Repository` if opening the Repository DB failed.
    #[tracing::instrument(level = "debug")]
    pub fn repository_db(&self) -> Result<crate::repository_db::RepositoryDb> {
        crate::repository_db::RepositoryDb::open(&self.repository_dir)
    }
}
