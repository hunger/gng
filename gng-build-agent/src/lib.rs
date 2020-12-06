// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! `gng-build` functionality

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
// - Error Handling:
// ----------------------------------------------------------------------

/// `Error` type for the `gng_shared` library
#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
    /// Script error.
    #[error("Script error: {0}, caused by: {1}")]
    Script(String, String),

    /// Conversion error.
    #[error("Conversion error: {0}.")]
    Conversion(String),

    /// Not sure what actually went wrong...
    #[error("unknown error")]
    Unknown,
}

/// `Result` type for the `gng_shared` library
pub type Result<T> = std::result::Result<T, Error>;

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

pub mod engine;
pub mod source_packet;
