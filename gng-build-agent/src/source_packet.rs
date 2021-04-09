// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePacket` and related code

use eyre::WrapErr;

// This does not use Serde since the error reporting is not that good there!

// - Helpers:
// ----------------------------------------------------------------------

fn has_function(engine: &mut crate::engine::Engine, expression: &str) -> eyre::Result<()> {
    if engine.has_function(expression) {
        Ok(())
    } else {
        Err(eyre::eyre!(format!(
            "No \"{}\" function found in Lua build file.",
            expression
        )))
    }
}

/// Create a `SourcePacket` from an `Engine`
///
/// # Errors
/// Passes along `Error::Script` from the evaluation
pub fn from_engine(
    engine: &mut crate::engine::Engine,
) -> eyre::Result<gng_build_shared::SourcePacket> {
    has_function(engine, "prepare")?;
    has_function(engine, "build")?;
    has_function(engine, "check")?;
    has_function(engine, "install")?;
    has_function(engine, "polish")?;

    engine
        .evaluate::<gng_build_shared::SourcePacket>("PKG")
        .wrap_err("Failed to get result table from lua build script")
}
