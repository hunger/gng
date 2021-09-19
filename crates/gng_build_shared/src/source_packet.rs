// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use gng_core::{Name, Names, Version};

// ----------------------------------------------------------------------
// - Source:
// ----------------------------------------------------------------------

/// A `Source` that needs building
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourceDefinition {
    /// A `Url` To get the `Source` from
    pub source: String,
    /// A list of possible mirrors to download from
    #[serde(default)]
    pub mirrors: Vec<String>,

    /// The file or directory name to create
    #[serde(default)]
    pub destination: String,

    /// Does this source file need unpacking?
    pub unpack: bool,
    // /// Validation values:
    // pub hash: Hash,
}

impl std::fmt::Display for SourceDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Source @\"{}\" -> \"{}\"", self.source, self.destination)
    }
}

// ----------------------------------------------------------------------
// - FacetDefinition:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct FacetDefinition {
    /// The `description_suffix` appended to packet descriptions
    pub description_suffix: String,
    /// The packet description
    #[serde(default)]
    pub mime_types: Vec<String>,
    /// Glob-patterns for `files` to include in the `Packet`
    #[serde(default)]
    pub files: Vec<String>,
}

// ----------------------------------------------------------------------
// - PacketDefinition:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct PacketDefinition {
    /// The `name` of the Packet.
    pub name: Name,
    /// The packet description
    pub description: String,
    /// The `dependencies` of the `Packet`
    #[serde(default)]
    pub dependencies: Names,

    /// Glob-patterns for `files` to include in the `Packet`
    #[serde(default)]
    pub files: Vec<String>,

    /// The `FacetDefinition`
    pub facet: Option<FacetDefinition>,
}

// ----------------------------------------------------------------------
// - SourcePacket:
// ----------------------------------------------------------------------

/// A description of a `SourcePacket`
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourcePacket {
    /// `name` of the sources
    pub name: Name,
    /// `description` of the source packet.
    pub description: String,
    /// `version`
    pub version: Version,
    /// `license`
    pub license: String,
    /// `url`
    pub url: String,
    /// `bug_url`
    pub bug_url: String,

    /// Enable `bootstrap` support in the build container.
    pub bootstrap: bool,

    /// `build_dependencies` of the source packet.
    pub build_dependencies: Names,
    /// `check_dependencies` of the source packet.
    pub check_dependencies: Names,
    /// The `sources` to build.
    pub sources: Vec<SourceDefinition>,
    /// The different `packets` to generate from the sources.
    pub packets: Vec<PacketDefinition>,
}

impl std::fmt::Display for SourcePacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}@{}\"", self.name, self.version)
    }
}
