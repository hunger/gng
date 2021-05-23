// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{Error, Result};

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

const REPOSITORY_DIR_KEY: &str = "repository_dir";
const REMOTES_DIR_KEY: &str = "remotes_dir";
const LOCALS_DIR_KEY: &str = "locals_dir";
const PACKET_DB_DIR_KEY: &str = "packet_db_dir";

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn env_path(env_var: &str, default_prefix: &str, fallback: &str) -> std::path::PathBuf {
    std::env::var(env_var).map_or_else(
        |_| {
            std::env::var("HOME").map_or_else(
                |_| std::path::PathBuf::from(fallback),
                |p| std::path::PathBuf::from(p).join(default_prefix),
            )
        },
        std::path::PathBuf::from,
    )
}

fn config_path() -> std::path::PathBuf {
    env_path("XDG_CONFIG_HOME", ".config", "/etc")
}

fn cache_path() -> std::path::PathBuf {
    env_path("XDG_CACHE_HOME", ".cache", "/var/cache")
}

fn data_path() -> std::path::PathBuf {
    env_path("XDG_DATA_HOME", ".local/share", "/var/lib")
}

// Fill in default config file path if possible
fn default_config_file_path(
    config_file: &Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    if config_file.is_some() {
        config_file.clone()
    } else {
        let config_file_path = config_path().join("gng/config");
        if config_file_path.exists() {
            Some(config_file_path)
        } else {
            None
        }
    }
}

fn create_config_file_parser(
    config_file: &Option<std::path::PathBuf>,
) -> Result<justconfig::Config> {
    let mut conf = justconfig::Config::default();

    // Open the configuration file
    if let Some(config_file) = &config_file {
        let file = std::fs::File::open(config_file)?;
        conf.add_source(
            justconfig::sources::text::ConfigText::new(
                file,
                config_file.to_string_lossy().as_ref(),
            )
            .map_err(|e| Error::Config(e.to_string()))?,
        );
    };

    // Allow some environment variables to override configuration values read
    // from the configuration file.
    let config_env = justconfig::sources::env::Env::new(&[
        (
            justconfig::ConfPath::from(&[REPOSITORY_DIR_KEY]),
            std::ffi::OsStr::new("GNG_REPOSITORY_DIR"),
        ),
        (
            justconfig::ConfPath::from(&[REMOTES_DIR_KEY]),
            std::ffi::OsStr::new("GNG_REMOTES_DIR"),
        ),
        (
            justconfig::ConfPath::from(&[LOCALS_DIR_KEY]),
            std::ffi::OsStr::new("GNG_LOCALS_DIR"),
        ),
        (
            justconfig::ConfPath::from(&[PACKET_DB_DIR_KEY]),
            std::ffi::OsStr::new("GNG_PACKET_DB_DIR"),
        ),
    ]);
    conf.add_source(config_env);

    // Defaults:
    let mut defaults = justconfig::sources::defaults::Defaults::default();
    defaults.set(
        conf.root().push_all(&[REPOSITORY_DIR_KEY]),
        config_path()
            .join("gng/repositories")
            .to_string_lossy()
            .as_ref(),
        "default",
    );
    defaults.set(
        conf.root().push_all(&[REMOTES_DIR_KEY]),
        cache_path().join("gng/remotes").to_string_lossy().as_ref(),
        "default",
    );
    defaults.set(
        conf.root().push_all(&[LOCALS_DIR_KEY]),
        data_path().join("gng/locals").to_string_lossy().as_ref(),
        "default",
    );
    defaults.set(
        conf.root().push_all(&[PACKET_DB_DIR_KEY]),
        cache_path().join("gng/packets").to_string_lossy().as_ref(),
        "default",
    );
    conf.add_source(defaults);

    Ok(conf)
}

// ----------------------------------------------------------------------
// - Config:
// ----------------------------------------------------------------------

/// A Configuration object that can be read from a configuration file.
#[derive(Debug)]
pub struct Config {
    /// The directory holding repository definition files
    pub repository_dir: std::path::PathBuf,
    /// The directory holding cached data from remote repositories
    pub remotes_dir: std::path::PathBuf,
    /// The directory containing all data from local repositories
    pub locals_dir: std::path::PathBuf,
    /// The directory holding the packet DB
    pub packet_db_directory: std::path::PathBuf,
}

impl Config {
    /// Parse a config file and create a `Config` object from it.
    ///
    /// # Errors
    /// May return an `Error::Config` if opening the `config_file` or parsing the necessary values fails.
    #[tracing::instrument(level = "debug")]
    pub fn new(config_file: &Option<std::path::PathBuf>) -> Result<Self> {
        use justconfig::item::ValueExtractor;

        // Fill in default config file
        let config_file = default_config_file_path(config_file);

        let conf = create_config_file_parser(&config_file)?;

        Ok(Self {
            repository_dir: conf
                .get(conf.root().push(REPOSITORY_DIR_KEY))
                .value()
                .map_err(|e| Error::Config(e.to_string()))?,
            remotes_dir: conf
                .get(conf.root().push(REMOTES_DIR_KEY))
                .value()
                .map_err(|e| Error::Config(e.to_string()))?,
            locals_dir: conf
                .get(conf.root().push(LOCALS_DIR_KEY))
                .value()
                .map_err(|e| Error::Config(e.to_string()))?,
            packet_db_directory: conf
                .get(conf.root().push(PACKET_DB_DIR_KEY))
                .value()
                .map_err(|e| Error::Config(e.to_string()))?,
        })
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
