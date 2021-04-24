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
    /// A `Error` triggered by the storage backend.
    #[error("Repository backend error.")]
    Backend(#[from] sled::Error),

    /// A `Error` about trying to work with a DB not set up by gng.
    #[error("Backend DB does not look like a GnG repository!")]
    WrongMagic,

    /// A `Error` about invalid DB schema.
    #[error("Repository backend uses an unsupported schema. Please upgrade your gng tools!")]
    WrongSchema,

    /// Not sure what actually went wrong...
    #[error("unknown error")]
    Unknown,
}

/// `Result` type for the `gng_shared` library
pub type Result<T> = std::result::Result<T, Error>;

// ----------------------------------------------------------------------
// - Modules:
// ----------------------------------------------------------------------

pub mod repository;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use repository::Repository;

/// Open a `Repository`
///
/// # Errors
///  * `Error::WrongSchema` if the repository does not use a supported schema version
///  *`Error::Backend` if the Backend has trouble reading the repository data
pub fn open(path: &std::path::Path) -> Result<impl Repository> {
    repository::RepositoryImpl::new(path)
}
