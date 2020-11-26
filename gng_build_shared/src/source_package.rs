// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! `A source package configuration

// FIXME: Find a better place for this!

use gng_shared::package::GpgKeyId;
use gng_shared::package::Name;
use gng_shared::package::Url;

/// A supported hashing algorithm:
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HashAlgorithm {
    /// No hash validation needed
    NONE(),
    /// SHA 256
    SHA256([u8; 32]),
    /// SHA 512
    SHA512([u8; 64]),
}

/// A `Hash`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hash {
    /// The `HashAlgorithm` used by this hash
    pub algorithm: HashAlgorithm,
    /// The value of the hash operation:
    pub value: HashValue,
}

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
