// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::{GpgKeyId, Hash, Name, Version};

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
    pub mirrors: Vec<String>,

    /// Does this source file need unpacking?
    pub unpack: bool,

    /// A set of GPG keys used to sign `Source`
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
    /// A `suffix` to append to the `source_name` to get the `package_name`
    pub suffix: String,
    /// The package description
    pub description: String,
    /// The `dependencies` of the `Package`
    pub dependencies: Vec<Name>,
    /// `optional_dependencies` of the `Package`
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
// - Build stage:
// ----------------------------------------------------------------------

/// The stage of the build
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum BuildStage {
    /// A foreign bootstrap environment needs to be available
    ForeignBootstrap,
    /// A native bootstrap environment needs to be available
    NativeBootstrap,
    /// No bootstrap environment is needed
    Native,
}

impl Default for BuildStage {
    fn default() -> Self {
        Self::Native
    }
}

// ----------------------------------------------------------------------
// - SourcePacket:
// ----------------------------------------------------------------------

/// A description of a `SourcePacket`
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourcePacket {
    /// `source_name`
    pub source_name: Name,
    /// `version`
    pub version: Version,
    /// `license`
    pub license: String,
    /// `url`
    pub url: Option<String>,
    /// `bug_url`
    pub bug_url: Option<String>,

    /// The `BuildStage` to apply
    pub build_stage: BuildStage,

    /// `build_dependencies`
    pub build_dependencies: Vec<Name>,
    /// `check_dependencies`
    pub check_dependencies: Vec<Name>,

    /// `sources`
    pub sources: Vec<Source>,
    /// `packets`
    pub packets: Vec<PacketDefinition>,
}

impl std::fmt::Display for SourcePacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}@{}\"", self.source_name, self.version)
    }
}
