// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Functionality related to creating a new packet

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

use eyre::{eyre, WrapErr};

// ----------------------------------------------------------------------
// - Modules:
// ----------------------------------------------------------------------

pub(crate) mod deterministic_directory_iterator;
pub mod filter;
pub(crate) mod packager;
pub(crate) mod path;
pub(crate) mod storage_function;

// Re-export:
pub use gng_packet_io::{BinaryFacetDefinition, BinaryPacketDefinition};

// ----------------------------------------------------------------------
// - Structures:
// ----------------------------------------------------------------------

pub use gng_core::{Name, Names, Version};

use std::rc::Rc;

/// A definition of one `Packet`
pub struct PacketDefinition {
    data: gng_packet_io::BinaryPacketDefinition,

    merged_facets: Names,
    filter: Rc<dyn filter::Filter>,
    is_empty: bool,
}

impl PacketDefinition {
    /// Constructor
    pub fn new(
        data: gng_packet_io::BinaryPacketDefinition,
        merged_facets: Names,
        filter: Rc<dyn filter::Filter>,
        is_empty: bool,
    ) -> Self {
        Self {
            data,
            merged_facets,
            filter,
            is_empty,
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
    package_usr_directory: &std::path::Path,
    packets: &[PacketDefinition],
    facets: &[FacetDefinition],
) -> eyre::Result<Vec<std::path::PathBuf>> {
    if packets.is_empty() || facets.is_empty() {
        tracing::warn!("Packet generation SKIPPED: No packets/facets, so nothing to do.");
        return Ok(Vec::new());
    }

    tracing::info!(
        "Packaging \"{}\".",
        &package_usr_directory.to_string_lossy()
    );

    let mut packager = crate::packager::create_packager(packets, facets)?;

    for it in crate::deterministic_directory_iterator::DeterministicDirectoryIterator::new(
        package_usr_directory,
    )? {
        packager.package(&it?)?;
    }

    packager.finish()
}

/// Turn a `String` slice into a `Vec<glob::Pattern>`
///
/// # Errors
///
/// Return an error if one of the input `String`s was not a valid pattern.
pub fn strings_to_globs(globs: &[String]) -> eyre::Result<Vec<glob::Pattern>> {
    globs
        .iter()
        .map(|s| glob::Pattern::new(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| eyre!("Failed to create GLOB pattern: {}", e))
}

/// Turn a `String` slice into a `Vec<regex::RegEx>`
///
/// # Errors
///
/// Return an error if one of the input `String`s was not a valid pattern.
pub fn strings_to_regex(regex: &[String]) -> eyre::Result<Vec<regex::Regex>> {
    regex
        .iter()
        .map(|s| regex::Regex::new(s))
        .collect::<Result<Vec<_>, _>>()
        .wrap_err("Failed to create RegEx.")
}
