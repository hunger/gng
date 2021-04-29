// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Repository management for all `gng` binaries.

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

use std::cmp::Ordering;

// ----------------------------------------------------------------------
// - ErrorHandling:
// ----------------------------------------------------------------------

/// Repository related Errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A `Error` triggered by the storage backend.
    #[error("Repository backend error.")]
    Backend(#[from] sled::Error),

    /// A `Error` about trying to work with a DB not set up by gng.
    #[error("Backend DB does not look like a GnG repository!")]
    WrongMagic,

    /// A `Error` about invalid DB schema.
    #[error("Repository backend uses an unsupported schema. Please upgrade your gng tools!")]
    WrongSchema,

    /// A `Error` related to a `Repository`
    #[error("Repository Error: {}", .0)]
    Repository(String),

    /// Not sure what actually went wrong...
    #[error("unknown error")]
    Unknown,
}

/// `Result` type for the `gng_shared` library
pub type Result<T> = std::result::Result<T, Error>;

// ----------------------------------------------------------------------
// - Modules:
// ----------------------------------------------------------------------

pub mod repository_db;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use repository_db::RepositoryDb;

pub use uuid::Uuid; // Reexport Uuid from uuid crate!

// ----------------------------------------------------------------------
// - Structures:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
/// Data on a repository of `Packet`s.
pub struct Repository {
    /// The user-visible name of this repository
    pub name: gng_shared::Name,
    /// The repository UUID
    pub uuid: crate::Uuid,
    /// The priority of this `RepositoryData`
    pub priority: u32,
    /// The url to pull updates from
    pub pull_url: Option<String>,
    /// The base URL to download `Packet`s from
    pub packet_base_url: String,
    /// The base directory holding the source packages for this repository.
    pub sources_base_directory: Option<std::path::PathBuf>,
    /// `Repository` this one depends on
    pub dependencies: gng_shared::Names,
    /// `A list of tags applied to this `Repository`.
    /// `Packet` names must be unique across all repositories sharing a tag!
    pub tags: gng_shared::Names,
}

impl Eq for Repository {}

impl Ord for Repository {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.uuid.cmp(&other.uuid),
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
        }
    }
}

impl PartialEq for Repository {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl PartialOrd for Repository {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ----------------------------------------------------------------------
// - Functions:
// ----------------------------------------------------------------------

/// Open a `Repository`
///
/// # Errors
///  * `Error::WrongSchema` if the repository does not use a supported schema version
///  *`Error::Backend` if the Backend has trouble reading the repository data
#[tracing::instrument(level = "trace")]
pub fn open(path: &std::path::Path) -> Result<impl RepositoryDb> {
    let config = sled::Config::default().path(path.to_owned());

    repository_db::RepositoryDbImpl::new(config.open().map_err(crate::Error::Backend)?)
}
