// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePacket` and related code

use gng_shared::{Name, Version};

use std::convert::TryFrom;

// - Helpers:
// ----------------------------------------------------------------------

fn error(expression: &str, reason: &str) -> gng_shared::Error {
    gng_shared::Error::Script {
        message: format!("Evaluation of \"{}\" failed: {}", expression, reason),
    }
}

fn map_error(e: gng_shared::Error, expression: &str) -> gng_shared::Error {
    match e {
        gng_shared::Error::Script { message } => error(expression, &message),
        _ => error(expression, "Unknown error"),
    }
}

fn name_from_expression(
    engine: &mut crate::engine::Engine,
    expression: &str,
) -> gng_shared::Result<Name> {
    let name = engine
        .evaluate::<String>("PKG.name")
        .map_err(|e| map_error(e, expression))?;
    Name::try_from(name).map_err(|e| map_error(e, expression))
}

/// Create a `SourcePacket` from an `Engine`
///
/// # Errors
/// Passes along `Error::Script` from the evaluation
pub fn from_engine(
    engine: &mut crate::engine::Engine,
) -> gng_shared::Result<gng_build_shared::SourcePacket> {
    Ok(gng_build_shared::SourcePacket {
        name: name_from_expression(engine, "PKG.name")?,
        version: Version::new(0, "foo", "bar")?,
        license: "".to_string(),
        url: None,
        bug_url: None,
        bootstrap: false,
        build_dependencies: vec![],
        check_dependencies: vec![],
        sources: vec![],
        packets: vec![],
    })
}
