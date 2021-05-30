// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Repository management for all `gng` binaries.

// Features:
#![feature(map_try_insert)]
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

pub mod gng_ext;
pub mod packet_db;
pub mod repository_db;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use uuid::Uuid; // Reexport Uuid from uuid crate!

pub use gng_ext::GngDbExt;
pub use packet_db::PacketDb;
pub use repository_db::RepositoryDb;

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
    #[serde(default)]
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
    #[serde(default)]
    pub packets_url: Option<String>,
}

/// The source of all data in a `Repository`
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum RepositorySource {
    /// A `Local` repository that users can add packets to
    #[serde(rename = "local")]
    Local(LocalRepository),
    /// A `Remote` repository hosted elsewhere
    #[serde(rename = "remote")]
    Remote(RemoteRepository),
}

/// The relations between a `Repository` and other `Repository`s.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum RepositoryRelation {
    /// Override another `Repository`
    #[serde(rename = "override")]
    Override(Uuid),
    /// Depend on zero or more other `Repository`s.
    #[serde(rename = "dependencies")]
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
    #[serde(default)]
    pub priority: u32,

    /// `Repository`(s) this one relates to
    #[serde(flatten)]
    pub relation: RepositoryRelation,

    /// The `RepositoryConnectivity` we are dealing with plus all
    /// the kind-specific data.
    /// Basically: Where does all the data in this `Repository` come from?
    #[serde(flatten)]
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
        let relation_str = match &self.relation {
            RepositoryRelation::Override(o) => {
                format!("\n    Override {}", o)
            }
            RepositoryRelation::Dependency(d) => {
                format!("\n    Depends on: {:?}", d)
            }
        };

        let sources_str = match &self.source {
            RepositorySource::Local(lr) => {
                let export = match &lr.export_directory {
                    Some(ed) => format!(" => {}", ed.to_string_lossy()),
                    _ => String::new(),
                };
                format!(
                    "\n    LOCAL  -- sources: {}{}",
                    &lr.sources_base_directory.to_string_lossy(),
                    export
                )
            }
            RepositorySource::Remote(rr) => {
                let packets = match &rr.packets_url {
                    Some(pu) => format!(" [packets at {}]", &pu),
                    _ => String::new(),
                };
                format!("\n    REMOTE -- {}{}", &rr.remote_url, &packets)
            }
        };
        format!(
            "{}: {} [{}]{}{}",
            &self.priority, &self.name, &self.uuid, relation_str, sources_str,
        )
    }

    /// Is this a local repository?
    #[must_use]
    pub const fn is_local(&self) -> bool {
        matches!(self.source, crate::RepositorySource::Local(_))
    }

    /// Does this repository override some other repository?
    #[must_use]
    pub const fn is_override(&self) -> bool {
        matches!(self.relation, crate::RepositoryRelation::Override(_))
    }
}

impl PartialEq for Repository {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for Repository {}

impl PartialOrd for Repository {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Repository {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => self.uuid.cmp(&other.uuid),
            std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
        }
    }
}
