// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePacket` and related code

use gng_build_shared::{PacketDefinition, Source, SourcePacket};
use gng_shared::{Name, Version};

// - Helpers:
// ----------------------------------------------------------------------

/// Create a `SourcePacket` from an `Engine`
pub fn from_engine(
    engine: &mut crate::engine::Engine,
) -> crate::Result<gng_build_shared::SourcePacket> {
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
