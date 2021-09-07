// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Write gng packages

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

pub mod packet_writer;
pub(crate) mod packet_writer_impl;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use packet_writer::{create_packet_writer, PacketWriter};
