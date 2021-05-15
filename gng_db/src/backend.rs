// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A backend database for a `Db`

use crate::{Error, Repository, Result};

use gng_shared::Name;

use std::convert::TryFrom;

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

const META_FILE: &str = "gng-db.json";

const MAGIC: &str = "1e925d91-3294-4676-add6-917376d89d58";
const SCHEMA_VERSION: u32 = 1;

const REPO_EXTENSION: &str = "repo";

// ----------------------------------------------------------------------
// - MetaData:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MetaData {
    magic: crate::Uuid,
    schema: u32,
}

impl MetaData {
    pub fn has_valid_magic(&self) -> bool {
        self.magic == crate::Uuid::parse_str(MAGIC).expect("UUID constant needs to be valid!")
    }

    pub const fn latest_schema() -> u32 {
        SCHEMA_VERSION
    }

    pub const fn current_schema(&self) -> u32 {
        self.schema
    }
}

impl Default for MetaData {
    fn default() -> Self {
        Self {
            magic: crate::Uuid::parse_str(MAGIC).expect("UUID constant needs to be valid!"),
            schema: SCHEMA_VERSION,
        }
    }
} // Default for MetaData

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn get_db_schema_version(db_directory: &std::path::Path) -> crate::Result<u32> {
    if !db_directory.exists() {
        return Err(crate::Error::Db(format!(
            "\"{}\" does not exist.",
            db_directory.to_string_lossy()
        )));
    }
    if !db_directory.is_dir() {
        return Err(crate::Error::Db(format!(
            "\"{}\" is not a directory.",
            db_directory.to_string_lossy()
        )));
    }

    let meta_file = db_directory.join(META_FILE);
    if !meta_file.exists() && std::fs::read_dir(db_directory)?.count() != 0 {
        return Err(crate::Error::Db(format!(
            "\"{}\" is not empty and has no meta.json file.",
            db_directory.to_string_lossy()
        )));
    }
    if meta_file.exists() && !meta_file.is_file() {
        return Err(crate::Error::Db(format!(
            "\"{}\" is not a file.",
            meta_file.to_string_lossy()
        )));
    }
    if meta_file.exists() && meta_file.metadata()?.len() > 1024 {
        return Err(crate::Error::Db(format!(
            "\"{}\" is too big.",
            meta_file.to_string_lossy()
        )));
    }

    let meta_data = if meta_file.exists() {
        let mf = std::fs::OpenOptions::new().read(true).open(&meta_file)?;
        serde_json::from_reader(&mf)
            .map_err(|_| crate::Error::Db("Failed to parse meta file.".to_string()))?
    } else {
        MetaData::default()
    };

    if !meta_data.has_valid_magic() {
        return Err(crate::Error::Db("Magic was not valid.".to_string()));
    }

    Ok(meta_data.current_schema())
}

fn read_repositories(db_directory: &std::path::Path) -> Result<Vec<Repository>> {
    let mut result = Vec::new();

    if let Ok(repository_files) = std::fs::read_dir(db_directory) {
        for entry in repository_files {
            match entry {
                Ok(entry) => {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name
                        .rsplit('.')
                        .next()
                        .map(|ext| ext.eq_ignore_ascii_case(REPO_EXTENSION))
                        != Some(true)
                    {
                        tracing::debug!("\"{}\" is not a .repo file, skipping.", file_name);
                        continue; // ignore non-json files!
                    }
                    let end = file_name.len() - 5;
                    Name::try_from(&file_name[..end]).map_err(|_| {
                        Error::Repository(format!("Invalid repository name in \"{}\".", &file_name))
                    })?;

                    let fd = std::fs::OpenOptions::new()
                        .read(true)
                        .open(entry.path())
                        .map_err(|e| {
                            Error::Repository(format!(
                                "Failed to open repository data \"{}\": {}.",
                                file_name, e
                            ))
                        })?;
                    result.push(serde_json::from_reader(fd).map_err(|e| {
                        Error::Repository(format!("Failed to parse \"{}\": {}.", file_name, e))
                    })?);
                }
                Err(e) => tracing::warn!("Could not retrieve directory data: {}", e),
            }
        }
    }

    Ok(result)
}

#[tracing::instrument(level = "trace")]
pub fn init_db(db_directory: &std::path::Path) -> crate::Result<()> {
    // FIXME: Add locking for DB access!

    tracing::debug!("Initializing DB.");
    std::fs::create_dir_all(db_directory).map_err(|e| {
        Error::Db(format!(
            "Can not initialize DB at \"{}\": {}.",
            db_directory.to_string_lossy(),
            e
        ))
    })?;

    let meta_file = db_directory.join(META_FILE);
    if meta_file.exists() && !meta_file.is_file() {
        return Err(Error::Db(format!(
            "Metadata exists in \"{}\", but is not a file.",
            db_directory.to_string_lossy(),
        )));
    }

    if !meta_file.exists() {
        tracing::debug!("Creating Meta data file.");

        let meta_fd = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(meta_file)
            .map_err(|e| {
                Error::Db(format!(
                    "Can not open meta info of DB at \"{}\": {}.",
                    db_directory.to_string_lossy(),
                    e
                ))
            })?;

        serde_json::to_writer(meta_fd, &MetaData::default()).map_err(|e| {
            Error::Db(format!(
                "Can not write meta info of DB at \"{}\": {}.",
                db_directory.to_string_lossy(),
                e
            ))
        })?;
    }

    Ok(())
}

#[tracing::instrument(level = "trace")]
pub fn read_db(db_directory: &std::path::Path) -> crate::Result<(Vec<Repository>,)> {
    // FIXME: Add locking for DB access!

    if get_db_schema_version(db_directory)? != MetaData::latest_schema() {
        return Err(crate::Error::Db("Unsupported schema version.".to_string()));
    }

    tracing::debug!("Reading repositories from DB.");
    let repositories = read_repositories(db_directory)?;

    Ok((repositories,))
}

fn repository_file_name(
    db_directory: &std::path::Path,
    repository_name: &Name,
) -> std::path::PathBuf {
    db_directory.join(format!("{}.{}", repository_name, REPO_EXTENSION))
}

#[tracing::instrument(level = "trace")]
pub fn persist_repository(
    db_directory: &std::path::Path,
    repository: &Repository,
) -> crate::Result<()> {
    // FIXME: Add locking for DB access!

    let repository_name = &repository.name;
    tracing::debug!("Persisting repository \"{}\".", &repository_name);

    let repository_file = repository_file_name(db_directory, repository_name);
    let rf = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&repository_file)
        .map_err(|e| {
            Error::Db(format!(
                "Failed to persist repository \"{}\": {}.",
                repository_name, e,
            ))
        })?;

    serde_json::to_writer_pretty(&rf, repository).map_err(|e| {
        Error::Db(format!(
            "Failed to persist repository \"{}\": {}",
            repository_name, e
        ))
    })
}

#[tracing::instrument(level = "trace")]
pub fn remove_repository(
    db_directory: &std::path::Path,
    repository_name: &gng_shared::Name,
) -> Result<()> {
    // FIXME: Add locking for DB access!

    tracing::debug!("Removing repository \"{}\".", &repository_name);

    let repository_file = repository_file_name(db_directory, repository_name);
    std::fs::remove_file(&repository_file)
        .map_err(|e| Error::Repository(format!("Failed to remove repository file: {}", e)))
}
