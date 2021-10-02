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
#![allow(
    clippy::non_ascii_literal,
    clippy::module_name_repetitions,
    clippy::let_unit_value
)]

// ----------------------------------------------------------------------
// - Modules:
// ----------------------------------------------------------------------

pub mod repository;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use repository::Repository;

// Reexport other crates:
pub use gng_packet_io::{
    BinaryFacet, BinaryFacetDefinition, BinaryFacetUsage, BinaryPacketDefinition,
};
