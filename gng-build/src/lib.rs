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
// - Modes:
// ----------------------------------------------------------------------

/// The `Mode` of operation
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// The `gng-build-agent` is run in `query` mode
    Query,
    /// The `gng-build-agent` is run in `prepare` mode
    Prepare,
    /// The `gng-build-agent` is run in `build` mode
    Build,
    /// The `gng-build-agent` is run in `check` mode
    Check,
    /// The `gng-build-agent` is run in `install` mode
    Install,
    /// The `gng-build-agent` is run in `package` mode
    Package,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Query
    }
}

impl Mode {
    /// The next mode to go to
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Query => Some(Self::Prepare), // default entry point
            Self::Prepare => Some(Self::Build),
            Self::Build => Some(Self::Check),
            Self::Check => Some(Self::Install),
            Self::Install => Some(Self::Package),
            Self::Package => None,
        }
    }
}

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

const DEFAULT_FACET_NAME: &str = "_MAIN_";

// ----------------------------------------------------------------------
// - ContentsPolicy:
// ----------------------------------------------------------------------

/// A policy defining whether a `Packet` needs to have contents or not.
#[derive(Debug, PartialEq)]
pub enum ContentsPolicy {
    /// The `Packet` must not contain any files or directories.
    Empty,
    /// The `Packet` may or may not contain files or directories.
    MaybeEmpty,
    /// The `Packet` must contain some files or directories.
    NotEmpty,
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

mod case_officer;
pub mod handler;
pub(crate) mod packager;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use case_officer::{CaseOfficer, CaseOfficerBuilder};
pub use packager::{Packager, PackagerBuilder};
