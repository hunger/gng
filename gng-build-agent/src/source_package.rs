// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePackage` and related code

use gng_shared::package::{Hash, Name, Url, Version};

use std::convert::TryFrom;

// - Helpers:
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// - Source Package:
// ----------------------------------------------------------------------

/// A description of a `SourcePackage`
#[derive(serde::Serialize)]
pub struct SourcePackage<'a> {
    #[serde(skip)]
    engine: crate::engine::Engine<'a>,

    source_name: Name,
    version: Version,
    license: String,
    url: Url,
    bug_url: Url,

    build_dependencies: Vec<Name>,
    check_dependencies: Vec<Name>,
}

impl<'a> SourcePackage<'a> {
    /// Create a new `SourcePackage`
    pub fn new(mut engine: crate::engine::Engine<'a>) -> eyre::Result<SourcePackage<'a>> {
        let source_name = engine.evaluate::<Name>("source_name")?;
        let version = engine.evaluate::<Version>("version")?;
        let license = engine.evaluate::<String>("license")?;
        let url = Url::try_from(engine.evaluate::<String>("url")?)?;
        let bug_url = Url::try_from(engine.evaluate::<String>("url")?)?;
        let build_dependencies = engine.evaluate_array::<Name>("build_dependencies")?;
        let check_dependencies = engine.evaluate_array::<Name>("check_dependencies")?;

        Ok(SourcePackage {
            engine,
            source_name,
            version,
            license,
            url,
            bug_url,
            build_dependencies,
            check_dependencies,
        })
    }
}

impl<'a> std::fmt::Display for SourcePackage<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.source_name, self.version)
    }
}

impl<'a> std::fmt::Debug for SourcePackage<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SOURCE PACKAGE DEBUG>")
    }
}
