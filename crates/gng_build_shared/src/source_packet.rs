// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

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
    pub source: String,
    /// A list of possible mirrors to download from
    #[serde(default)]
    pub mirrors: Vec<String>,

    /// The file or directory name to create
    #[serde(default)]
    pub destination: String,

    /// Does this source file need unpacking?
    #[serde(default = "always_true")]
    pub unpack: bool,
    // /// Validation values:
    // pub hash: Hash,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Source @\"{}\" -> \"{}\"", self.source, self.destination)
    }
}

// ----------------------------------------------------------------------
// - PacketDefinition:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct PacketDefinition {
    /// The `name` of the Packet.
    pub name: String,
    /// The packet description
    pub description: String,
    /// The `dependencies` of the `Packet`
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Glob-patterns for `files` to include in the `Packet`
    pub files: Vec<String>,
}

// ----------------------------------------------------------------------
// - SourcePacket:
// ----------------------------------------------------------------------

/// A description of a `SourcePacket`
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourcePacket {
    /// `name` of the sources
    pub name: String,
    /// `description` of the source packet.
    pub description: String,
    /// `version`
    pub version: String,
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
    pub build_dependencies: Vec<String>,
    /// `check_dependencies` of the source packet.
    #[serde(default)]
    pub check_dependencies: Vec<String>,
    /// The `sources` to build.
    pub sources: Vec<Source>,
    /// The different `packets` to generate from the sources.
    #[serde(default)]
    pub packets: Vec<PacketDefinition>,
}

impl Default for SourcePacket {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            version: String::new(),
            license: String::new(),
            url: None,
            bug_url: None,
            bootstrap: false,
            build_dependencies: Vec::new(),
            check_dependencies: Vec::new(),
            sources: Vec::new(),
            packets: Vec::new(),
        }
    }
} // Default for SourcePacket
impl std::fmt::Display for SourcePacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}@{}\"", self.name, self.version)
    }
}
