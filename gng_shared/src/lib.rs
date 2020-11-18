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
    /// Parsing a toml file failed.
    #[error("Parse error.")]
    ParseError(#[from] toml::de::Error),
    /// Conversion error.
    #[error("Conversion error: {0}")]
    ConversionError(&'static str),

    /// Not sure what actually went wrong...
    #[error("unknown error")]
    Unknown,
}

/// `Result` type for the `gng_shared` library
pub type Result<T> = std::result::Result<T, Error>;

pub mod config;
pub mod package;
