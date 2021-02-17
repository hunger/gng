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
    pub name: String,

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
        write!(f, "Source @{} -> {}", self.url, self.name)
    }
}

// ----------------------------------------------------------------------
// - PacketDefinition:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PacketDefinition {
    /// A `suffix` to append to the `name` to get the `package_name`
    #[serde(default)]
    pub suffix: String,
    /// The package description
    pub description: String,
    /// The `dependencies` of the `Package`
    #[serde(default)]
    pub dependencies: Vec<Name>,
    /// `optional_dependencies` of the `Package`
    #[serde(default)]
    pub optional_dependencies: Vec<Name>,
    /// Glob-patterns for `files` to include in the `Package`
    pub files: Vec<String>,
}

impl std::fmt::Display for PacketDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***PacketDefinition***")
    }
}

// ----------------------------------------------------------------------
// - SourcePacket:
// ----------------------------------------------------------------------

/// A description of a `SourcePacket`
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourcePacket {
    /// `name` of the sources
    pub name: Name,
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

    /// The `BuildStage` to apply
    #[serde(default = "always_false")]
    pub bootstrap: bool,

    /// `build_dependencies`
    pub build_dependencies: Vec<Name>,
    /// `check_dependencies`
    pub check_dependencies: Vec<Name>,

    /// `sources`
    #[serde(default)]
    pub sources: Vec<Source>,
    /// `packets`
    #[serde(default)]
    pub packets: Vec<PacketDefinition>,
}

impl std::fmt::Display for SourcePacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}@{}\"", self.name, self.version)
    }
}
