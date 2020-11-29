// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! `A source package configuration

use gng_shared::{GpgKeyId, Hash, Name, Url};

/// A description for a `SourcePackage`
pub struct SourcePackage {
    /// Source package `MetaData`

    /// The list of `Source`s to build:
    pub sources: Vec<Source>,

    /// The build-time only dependencies
    pub build_dependencies: Vec<Name>,
    /// The check-time only dependencies
    pub check_dependencies: Vec<Name>,
}
