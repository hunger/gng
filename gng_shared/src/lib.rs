// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Basic functionality for all `gng` binaries.

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

/// `Error` type for the `gng_shared` library
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Conversion error.
    #[error("Conversion error: {0}")]
    Conversion(&'static str),

    /// Not sure what actually went wrong...
    #[error("unknown error")]
    Unknown,
}

/// `Result` type for the `gng_shared` library
pub type Result<T> = std::result::Result<T, Error>;

/// Return `true` if the program is run by the `root` user.
pub fn is_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}

pub mod config;
pub mod package;
