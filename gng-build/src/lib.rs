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
    QUERY,
    /// The `gng-build-agent` is run in `prepare` mode
    PREPARE,
    /// The `gng-build-agent` is run in `build` mode
    BUILD,
    /// The `gng-build-agent` is run in `check` mode
    CHECK,
    /// The `gng-build-agent` is run in `install` mode
    INSTALL,
    /// The `gng-build-agent` is run in `package` mode
    PACKAGE,
}

impl Default for Mode {
    fn default() -> Self {
        Self::QUERY
    }
}

impl Mode {
    /// The next mode to go to
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::QUERY => Some(Self::PREPARE), // default entry point
            Self::PREPARE => Some(Self::BUILD), // default entry point
            Self::BUILD => Some(Self::CHECK),
            Self::CHECK => Some(Self::INSTALL),
            Self::INSTALL => Some(Self::PACKAGE),
            Self::PACKAGE => None,
        }
    }
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

mod case_officer;
pub mod message_handler;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use case_officer::{CaseOfficer, CaseOfficerBuilder};
