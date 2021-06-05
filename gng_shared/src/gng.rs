// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{Error, Result};

use clap::Clap;

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
// - Gng:
// ----------------------------------------------------------------------

/// A Configuration object that can be read from a configuration file.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Gng {
    /// The directory containing all data from local repositories
    pub locals_dir: PathBuf,
    /// The directory holding the packet DB
    pub db_directory: PathBuf,
    /// The directory holding cached data from remote repositories
    pub remotes_dir: PathBuf,
}

impl Default for Gng {
    fn default() -> Self {
        Self {
            locals_dir: data_path().join("gng/locals"),
            db_directory: data_path().join("gng/packets"),
            remotes_dir: cache_path().join("gng/remotes"),
        }
    }
}

impl Gng {
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
}

// ----------------------------------------------------------------------
// - GngArgs:
// ----------------------------------------------------------------------

/// Logging related arguments for command line parsing
#[derive(Debug, Clap)]
pub struct GngArgs {
    /// Set the output format for
    #[clap(long, display_order = 10000, env = "GNG_CONFIG_FILE")]
    config_file: Option<std::path::PathBuf>,
}

impl GngArgs {
    /// Install a default tracing subscriber
    ///
    /// # Errors
    /// a `crate::Error::Runtime` is returned if the setup fails
    pub fn create_gng(&self) -> crate::Result<Gng> {
        Gng::new(&self.config_file)
    }
}
