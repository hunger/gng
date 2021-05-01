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
    /// A generic `Error` about the `RepositoryDb`.
    #[error("General Repository DB error: {}", .0)]
    Db(String),

    /// An Io `Error`.
    #[error("IO error: {}", .0)]
    Io(#[from] std::io::Error),

    /// A `Error` related to a `Repository`
    #[error("Repository Error: {}", .0)]
    Repository(String),

    /// A `Error` related to a `Repository`
    #[error("Packet Error: {}", .0)]
    Packet(String),

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

// - Repository:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, Eq, serde::Deserialize, serde::Serialize)]
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

// - Packet:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
/// All the data on `Packet` needed to store it in a `Repository`.
pub struct PacketData {
    /// The `Facet`s defined for this `Packet`
    facets: Vec<(gng_shared::Name, gng_shared::Hash)>,
    /// The `Packet` data itself
    data: gng_shared::Packet,
    /// The `Hash` of the `Packet` itself
    hash: gng_shared::Hash,
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
    repository_db::RepositoryDbImpl::new(path)
}
