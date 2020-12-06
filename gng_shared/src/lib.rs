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

use std::os::unix::fs::PermissionsExt;

// ----------------------------------------------------------------------
// - Error Handling:
// ----------------------------------------------------------------------

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

// ----------------------------------------------------------------------
// - Functions:
// ----------------------------------------------------------------------

/// Return `true` if the program is run by the `root` user.
pub fn is_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}

/// Return `true` if the `path` is executable
pub fn is_executable(path: &std::path::Path) -> bool {
    match std::fs::metadata(path) {
        Err(_) => false,
        Ok(m) => (m.permissions().mode() & 0o111) != 0,
    }
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

pub mod config;
mod packet;

pub use packet::{GpgKeyId, Hash, Name, Packet, Version};
