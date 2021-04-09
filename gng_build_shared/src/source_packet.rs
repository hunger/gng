// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::{GpgKeyId, Hash, Name, Version};

//  - Helper:
//  ----------------------------------------------------------------------

const fn always_true() -> bool {
    true
}

const fn always_false() -> bool {
    false
}

const fn always_none_string() -> Option<String> {
    None
}

// ----------------------------------------------------------------------
// - Source:
// ----------------------------------------------------------------------

/// A `Source` that needs building
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Source {
    /// A `Url` To get the `Source` from
    pub url: String,

    /// The file or directory name to create
    #[serde(default)]
    pub directory: Option<String>,

    /// A list of possible mirrors to download from
    #[serde(default)]
    pub mirrors: Vec<String>,

    /// Does this source file need unpacking?
    #[serde(default = "always_true")]
    pub unpack: bool,

    /// A set of GPG keys used to sign `Source`
    #[serde(default)]
    pub signing_keys: Vec<GpgKeyId>,

    /// Validation values:
    pub hash: Hash,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_str = match &self.directory {
            None => String::new(),
            Some(d) => format!(" -> \"{}\"", d),
        };
        write!(f, "Source @\"{}\"{}", self.url, name_str)
    }
}

// ----------------------------------------------------------------------
// - Facet:
// ----------------------------------------------------------------------

/// `Facet` meta data
///
/// A `Facet` is some aspect of a `Packet` that should get separated from the rest.
/// This could be to reduce system size or to reduce dependencies of the main packet.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct Facet {
    /// A bit of text to append to the packet description
    pub description_suffix: String,
    /// Mime-types that should go into this `Facet`
    pub mime_types: Vec<String>,
    /// Glob patterns that cause matching files to go into this `Facet`
    pub patterns: Vec<String>,
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
    pub dependencies: Vec<Name>,
    /// Glob-patterns for `files` to include in the `Packet`
    pub files: Vec<String>,

    /// An optional `Facet` that will be used for dependent packages
    #[serde(default)]
    pub facet: Option<Facet>,
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
    #[serde(default = "always_none_string")]
    pub url: Option<String>,
    /// `bug_url`
    #[serde(default = "always_none_string")]
    pub bug_url: Option<String>,

    /// Enable `bootstrap` support in the build container.
    #[serde(default = "always_false")]
    pub bootstrap: bool,
    /// `build_dependencies` of the source packet.
    pub build_dependencies: Vec<Name>,
    /// `check_dependencies` of the source packet.
    #[serde(default)]
    pub check_dependencies: Vec<Name>,
    /// The `sources` to build.
    pub sources: Vec<Source>,
    /// The different `packets` to generate from the sources.
    #[serde(default)]
    pub packets: Vec<PacketDefinition>,
}

impl std::fmt::Display for SourcePacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}@{}\"", self.name, self.version)
    }
}
