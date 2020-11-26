// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! `A source package configuration

use gng_shared::package::{GpgKeyId, Hash, Name, Url};

/// A `Source` that needs building
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Source {
    /// A `Url` To get the `Source` from
    pub url: Url,

    /// Does this source file need unpacking?
    pub unpack: bool,

    /// A set of GPG keys used to sign `Source`
    pub signing_keys: Vec<GpgKeyId>,

    /// Validation values:
    pub hashes: Vec<Hash>,
}

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
