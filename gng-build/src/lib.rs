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
    /// The `gng-build-agent` is idle
    IDLE,
    /// The `gng-build-agent` is run in `query` mode
    QUERY,
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
    fn next(self) -> Self {
        match self {
            Mode::IDLE => Mode::IDLE,
            Mode::QUERY => Mode::BUILD, // default entry point
            Mode::BUILD => Mode::CHECK,
            Mode::CHECK => Mode::INSTALL,
            Mode::INSTALL => Mode::PACKAGE,
            Mode::PACKAGE => Mode::IDLE,
        }
    }
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

pub mod case_officer;
