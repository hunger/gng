// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePacket` and related code

use gng_build_shared::{PacketDefinition, Source};
use gng_shared::{Name, Version};

// - Helpers:
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// - Source Package:
// ----------------------------------------------------------------------

/// A description of a `SourcePacket`
#[derive(serde::Serialize)]
pub struct SourcePacket<'a> {
    #[serde(skip)]
    engine: crate::engine::Engine<'a>,

    source_name: Name,
    version: Version,
    license: String,
    url: Option<String>,
    bug_url: Option<String>,

    build_dependencies: Vec<Name>,
    check_dependencies: Vec<Name>,

    sources: Vec<Source>,
    packets: Vec<PacketDefinition>,
}

impl<'a> SourcePacket<'a> {
    /// Create a new `SourcePacket`
    pub fn new(mut engine: crate::engine::Engine<'a>) -> eyre::Result<SourcePacket<'a>> {
        let source_name = engine.evaluate::<Name>("source_name")?;
        let version = engine.evaluate::<Version>("version")?;
        let license = engine.evaluate::<String>("license")?;
        let url = engine.evaluate::<String>("url").unwrap_or(String::new());
        let bug_url = engine
            .evaluate::<String>("bug_url")
            .unwrap_or(String::new());
        let build_dependencies = engine.evaluate_array::<Name>("build_dependencies")?;
        let check_dependencies = engine.evaluate_array::<Name>("check_dependencies")?;

        let sources = engine.evaluate_array::<Source>("sources")?;
        let packets = engine.evaluate_array::<PacketDefinition>("packets")?;

        Ok(SourcePacket {
            engine,
            source_name,
            version,
            license,
            url: if url.is_empty() { None } else { Some(url) },
            bug_url: if bug_url.is_empty() {
                None
            } else {
                Some(bug_url)
            },
            build_dependencies,
            check_dependencies,
            sources,
            packets,
        })
    }

    /// Run the `prepare` function of the build script
    pub fn prepare(&mut self) -> crate::Result<()> {
        self.engine.call("prepare")?
    }

    /// Run the `build` function of the build script
    pub fn build(&mut self) -> crate::Result<()> {
        self.engine.call("build")?
    }

    /// Run the `check` function of the build script
    pub fn check(&mut self) -> crate::Result<()> {
        self.engine.call("check")?
    }

    /// Run the `install` function of the build script
    pub fn install(&mut self) -> crate::Result<()> {
        self.engine.call("install")?
    }
}

impl<'a> std::fmt::Display for SourcePacket<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.source_name, self.version)
    }
}
