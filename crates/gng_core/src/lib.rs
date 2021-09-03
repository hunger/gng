// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Basic functionality for all `gng` binaries.

// cSpell:ignore faccess

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

use std::path::{Path, PathBuf};

use faccess::PathExt;

// ----------------------------------------------------------------------
// - Error Handling:
// ----------------------------------------------------------------------

/// `Error` type for the `gng_core` library
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Configuration Error
    #[error("Configuration Error: {}", .0)]
    Config(String),

    /// Conversion error.
    #[error("Conversion error: Can not convert \"{expression}\" to {typename}: {message}.")]
    Conversion {
        /// The `expression` that could not get converted.
        expression: String,
        /// The `typename` that the `expression` failed to convert into.
        typename: String,
        /// A `message` describing why the conversion failed.
        message: String,
    },

    /// Script error.
    #[error("Script error: {message}")]
    Script {
        /// A `message` describing the error.
        message: String,
    },

    /// IO Error
    #[error("IO Error: {source}")]
    Io {
        /// The `std::io::Error` triggering this
        #[from]
        source: std::io::Error,
    },

    /// Path handling Error
    #[error("Path error: {source}")]
    Path {
        /// The `std::path::StripPrefixError` that caused this
        #[from]
        source: std::path::StripPrefixError,
    },

    /// Runtime Error
    #[error("Runtime Error: {message}")]
    Runtime {
        /// Error message.
        message: String,
    },

    /// Not sure what actually went wrong...
    #[error("unknown error")]
    Unknown,
}

/// `Result` type for the `gng_core` library
pub type Result<T> = std::result::Result<T, Error>;

// ----------------------------------------------------------------------
// - Functions:
// ----------------------------------------------------------------------

/// Find the canonical path to an executable or report an error
///
/// # Errors
/// Errors out when the executable con not be found or is not executable.
pub fn validate_executable(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize().map_err(|_| Error::Runtime {
        message: format!(
            "Failed to canonicalize executable path \"{}\".",
            path.to_string_lossy()
        ),
    })?;

    if !path.is_file() {
        Err(Error::Runtime {
            message: format!("Executable \"{}\" is not a file.", path.to_string_lossy()),
        })
    } else if path.executable() {
        Ok(path)
    } else {
        Err(Error::Runtime {
            message: format!(
                "Executable \"{}\" is not marked executable.",
                path.to_string_lossy()
            ),
        })
    }
}

/// Return `true` if the program is run by the `root` user.
#[must_use]
pub fn is_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}

/// Return `true` if all characters are lowercase 'a' to 'z', '0' to '9' or '_'
#[must_use]
pub fn all_name_chars(input: &str) -> bool {
    input
        .chars()
        .all(|c| ('a'..='z').contains(&c) || ('0'..='9').contains(&c) || (c == '_'))
}

/// Return `true` if all characters are lowercase 'a' to 'z', '0' to '9', '.' or '_'
#[must_use]
pub fn all_version_chars(input: &str) -> bool {
    input
        .chars()
        .all(|c| ('a'..='z').contains(&c) || ('0'..='9').contains(&c) || (c == '_') || (c == '.'))
}

/// Return `true` if all characters are (lc) hex digits or separators like '-', ' ' or '_'
#[must_use]
pub fn all_hex_or_separator(input: &str) -> bool {
    input.chars().all(|c| {
        ('0'..='9').contains(&c)
            || ('a'..='f').contains(&c)
            || (c == ' ')
            || (c == '-')
            || (c == '_')
    })
}

/// Return `true` if all characters are lowercase 'a' to 'z', '0' to '9', '.' or '_'
#[must_use]
pub fn start_alnum_char(input: &str) -> bool {
    input
        .chars()
        .take(1)
        .all(|c| ('a'..='z').contains(&c) || ('0'..='9').contains(&c))
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

pub mod log;

mod name;
pub use name::{Name, Names};

mod version;
pub use version::Version;
