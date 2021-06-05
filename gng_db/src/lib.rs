// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Repository management for all `gng` binaries.

// Features:
#![feature(map_try_insert)]
#![feature(get_mut_unchecked)]
// Setup warnings/errors:
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

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use uuid::Uuid; // Reexport Uuid from uuid crate!

pub use gng_ext::GngDbExt;
