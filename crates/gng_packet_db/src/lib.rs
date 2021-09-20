// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Functionality related to finding known packets.

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

use gng_core::Name;

// ----------------------------------------------------------------------
// - Trait:
// ----------------------------------------------------------------------

/// The actual packet DB.
pub trait Db {
    /// Check whether a packet/facet is known to this Db.
    fn knows(&self, packet: &Name, facet: &Option<Name>) -> bool;

    /// Get a path to a packet.
    ///
    /// # Errors
    ///
    /// Returns an error if the packet could not be found.
    fn find(&self, packet: &Name, facet: &Option<Name>) -> eyre::Result<std::path::PathBuf>;
}

// ----------------------------------------------------------------------
// - Modules:
// ----------------------------------------------------------------------

pub mod directory_db;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use directory_db::DirectoryDb;
