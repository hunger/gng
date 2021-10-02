// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Functionality related to running a command in a container

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::let_unit_value)]

use std::path::{Path, PathBuf};

// ----------------------------------------------------------------------
// - Error Handling:
// ----------------------------------------------------------------------

pub use gng_core::{Error, Result};

// ----------------------------------------------------------------------
// - Binding:
// ----------------------------------------------------------------------

/// A mapping of outside filesystem location to a in-container path
#[derive(Clone, Debug)]
pub struct BindMap {
    source: PathBuf,
    target: PathBuf,
}

/// A mapping for a overlay filesystem into the container
#[derive(Clone, Debug)]
pub struct OverlayMap {
    sources: Vec<PathBuf>,
    target: PathBuf,
}

/// A `Binding` definition for mount points
#[derive(Clone, Debug)]
pub enum Binding {
    /// A read/write binding
    RW(BindMap),
    /// A read only binding
    RO(BindMap),
    /// Put a tmpfs into the specified path inside the container
    TmpFS(PathBuf),
    /// Make a path inside the container inaccessible
    Inaccessible(PathBuf),
    /// Overlay some directory with another
    Overlay(OverlayMap),
    /// Overlay some directory with another
    OverlayRO(OverlayMap),
}

impl Binding {
    /// Create a new `RW` `Binding`
    #[must_use]
    pub fn rw<P1: Into<PathBuf>>(source: P1, target: &Path) -> Self {
        Self::RW(BindMap {
            source: source.into(),
            target: target.into(),
        })
    }

    /// Create a new `RO` `Binding`
    #[must_use]
    pub fn ro<P1: Into<PathBuf>>(source: P1, target: &Path) -> Self {
        Self::RO(BindMap {
            source: source.into(),
            target: target.into(),
        })
    }

    /// Create a new `TmpFS` `Binding`
    #[must_use]
    pub fn tmpfs(target: &Path) -> Self {
        Self::TmpFS(target.into())
    }

    /// Create a new `Inaccessible` `Binding`
    #[must_use]
    pub fn inaccessible(target: &Path) -> Self {
        Self::Inaccessible(target.into())
    }

    /// Create a new `Overlay` `Binding`
    #[must_use]
    pub fn overlay<P1: AsRef<std::ffi::OsStr>>(sources: &[P1], target: &Path) -> Self {
        Self::Overlay(OverlayMap {
            sources: sources.iter().map(|s| s.into()).collect(),
            target: target.into(),
        })
    }

    /// Create a new `OverlayRO` `Binding`
    #[must_use]
    pub fn overlay_ro<P1: AsRef<std::ffi::OsStr>>(sources: &[P1], target: &Path) -> Self {
        Self::Overlay(OverlayMap {
            sources: sources.iter().map(|s| s.into()).collect(),
            target: target.into(),
        })
    }
}

// - Modules:
// ----------------------------------------------------------------------

mod command;
pub use command::{Command, CommandBuilder};

mod runner;
pub use runner::{Runner, RunnerBuilder};
