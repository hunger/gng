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
        Mode::QUERY
    }
}

impl Mode {
    /// The next mode to go to
    pub fn next(self) -> Option<Self> {
        match self {
            Mode::QUERY => Some(Mode::PREPARE), // default entry point
            Mode::PREPARE => Some(Mode::BUILD), // default entry point
            Mode::BUILD => Some(Mode::CHECK),
            Mode::CHECK => Some(Mode::INSTALL),
            Mode::INSTALL => Some(Mode::PACKAGE),
            Mode::PACKAGE => None,
        }
    }
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

pub mod case_officer;
pub mod message_handler;
