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

// ----------------------------------------------------------------------
// - ErrorHandling:
// ----------------------------------------------------------------------

/// Repository related Errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A generic `Error` about the `RepositoryDb`.
    #[error("General repository DB error: {}", .0)]
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

/// Data on a `LocalRepository`
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct LocalRepository {
    /// The base directory holding the source packages for this repository.
    /// `gng` expects a folder below this directory with the name of a
    /// `Packet` to be built and will look for a `build.lua` in that folder.
    pub sources_base_directory: std::path::PathBuf,
    /// The directory to export this `Repository` into for use as a
    /// `Remote` repository.
    pub export_directory: Option<std::path::PathBuf>,
}

/// Data on a `RemoteRepository`
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RemoteRepository {
    /// The url to pull updates from
    pub remote_url: String,
    /// The base URL to download the packaging sources from.
    /// This is for information only and will not be used by the
    /// `Repository` this `RepositoryKind` is part of!
    pub packets_url: Option<String>,
}

/// The source of all data in a `Repository`
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum RepositorySource {
    /// A `Local` repository that users can add packets to
    Local(LocalRepository),
    /// A `Remote` repository hosted elsewhere
    Remote(RemoteRepository),
}

/// The relations between a `Repository` and other `Repository`s.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum RepositoryRelation {
    /// Override another `Repository`
    Override(Uuid),
    /// Depend on zero or more other `Repository`s.
    Dependency(Vec<Uuid>),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
/// Data on a repository of `Packet`s.
pub struct Repository {
    /// The user-visible name of this repository
    pub name: gng_shared::Name,
    /// The repository UUID
    pub uuid: Uuid,
    /// The priority of this `RepositoryData`
    pub priority: u32,
    /// `A list of tags applied to this `Repository`.
    /// `Packet` names must be unique across all repositories sharing a tag!
    pub tags: gng_shared::Names,

    /// `Repository`(s) this one relates to
    pub relation: RepositoryRelation,

    /// The `RepositoryConnectivity` we are dealing with plus all
    /// the kind-specific data.
    /// Basically: Where does all the data in this `Repository` come from?
    pub source: RepositorySource,
}

impl Repository {
    /// Return a single-line JSON representation of the value
    ///
    /// # Errors
    /// A `gng_shared::Error::Conversion` might be returned.
    pub fn to_json(&self) -> gng_shared::Result<String> {
        serde_json::to_string(&self).map_err(|e| gng_shared::Error::Conversion {
            expression: "Repository".to_string(),
            typename: "JSON".to_string(),
            message: e.to_string(),
        })
    }

    /// Return a multi-line pretty representation of the value for human consumption
    ///
    /// # Errors
    /// A `gng_shared::Error::Conversion` might be returned.
    #[must_use]
    pub fn to_pretty_string(&self) -> String {
        let tags_str = self
            .tags
            .into_iter()
            .map(gng_shared::Name::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let relation_str = match &self.relation {
            RepositoryRelation::Override(o) => {
                format!("\n    Override {}", o)
            }
            RepositoryRelation::Dependency(d) => {
                format!("\n    Depends on: {:?}", d)
            }
        };
        let sources_str = "";
        format!(
            "{}: {} [{}] -- tags: {}{}{}",
            &self.priority, &self.name, &self.uuid, tags_str, relation_str, sources_str,
        )
    }
}

impl PartialEq for Repository {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
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
