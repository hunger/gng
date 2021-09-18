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
// - Modules:
// ----------------------------------------------------------------------

pub(crate) mod deterministic_directory_iterator;
pub mod filter;
pub(crate) mod packager;
pub(crate) mod path;
pub(crate) mod storage_function;

// ----------------------------------------------------------------------
// - Structures:
// ----------------------------------------------------------------------

pub use gng_core::{Name, Names, Version};

use std::rc::Rc;

/// A definition of one `Packet`
pub struct PacketDefinition {
    name: Name,
    version: Version,
    merged_facets: Names,
    metadata: Vec<u8>,
    filter: Rc<dyn filter::Filter>,
}

impl PacketDefinition {
    /// Constructor
    pub fn new(
        name: Name,
        version: Version,
        merged_facets: Names,
        metadata: Vec<u8>,
        filter: Rc<dyn filter::Filter>,
    ) -> Self {
        Self {
            name,
            version,
            merged_facets,
            metadata,
            filter,
        }
    }
}

/// A definition of one `Facet`
pub struct FacetDefinition {
    name: Option<Name>,
    filter: Rc<dyn filter::Filter>,
}

impl FacetDefinition {
    /// Constructor
    pub fn new(name: Option<Name>, filter: Rc<dyn filter::Filter>) -> Self {
        Self { name, filter }
    }
}
// ----------------------------------------------------------------------
// - Functions:
// ----------------------------------------------------------------------

/// Package up the directory `package_root_directory`
///
/// # Errors
/// Error out if the `package_root` is not a directory.
#[tracing::instrument(level = "debug", skip(packets, facets))]
pub fn package(
    package_root_directory: &std::path::Path,
    packets: &[PacketDefinition],
    facets: &[FacetDefinition],
) -> eyre::Result<Vec<std::path::PathBuf>> {
    let usr_directory = package_root_directory.join("usr");
    tracing::info!("Packaging \"{}\".", &usr_directory.to_string_lossy());

    let mut packager = crate::packager::create_packager(packets, facets)?;

    for it in crate::deterministic_directory_iterator::DeterministicDirectoryIterator::new(
        &usr_directory,
    )? {
        packager.package(&it?)?;
    }

    packager.finish()
}

/// Turn a `&str` slice into a `Vec<glob::Pattern>`
///
/// # Errors
///
/// Return an error if one of the input `str` was not a valid pattern.
pub fn strings_to_globs(globs: &[&str]) -> eyre::Result<Vec<glob::Pattern>> {
    globs
        .iter()
        .map(|s| glob::Pattern::new(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| eyre::eyre!("Failed to  create GLOB pattern: {}", e))
}
