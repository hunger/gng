// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePacket` and related code

use gng_shared::{Name, Version};

// This does not use Serde since the error reporting is not that good there!

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

fn from_expression<T: serde::de::DeserializeOwned>(
    engine: &mut crate::engine::Engine,
    expression: &str,
) -> gng_shared::Result<T> {
    engine
        .evaluate::<T>(expression)
        .map_err(|e| map_error(e, expression))
}

fn converted_expression<T: std::convert::TryFrom<String, Error = gng_shared::Error>>(
    engine: &mut crate::engine::Engine,
    expression: &str,
) -> gng_shared::Result<T> {
    let name = from_expression::<String>(engine, expression)?;
    T::try_from(name)
}

fn url_option(engine: &mut crate::engine::Engine, expression: &str) -> Option<String> {
    let url = from_expression::<String>(engine, expression).unwrap_or_else(|_| String::new());
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn has_function(engine: &mut crate::engine::Engine, name: &str) -> gng_shared::Result<()> {
    if engine.has_function(name) {
        Ok(())
    } else {
        Err(gng_shared::Error::Script {
            message: format!("Function \"{}\" is missing.", name),
        })
    }
}

/// Create a `SourcePacket` from an `Engine`
///
/// # Errors
/// Passes along `Error::Script` from the evaluation
pub fn from_engine(
    engine: &mut crate::engine::Engine,
) -> gng_shared::Result<gng_build_shared::SourcePacket> {
    has_function(engine, "prepare")?;
    has_function(engine, "build")?;
    has_function(engine, "check")?;
    has_function(engine, "install")?;
    has_function(engine, "polish")?;

    Ok(gng_build_shared::SourcePacket {
        name: converted_expression::<Name>(engine, "PKG.name")?,
        version: converted_expression::<Version>(engine, "PKG.version")?,
        license: from_expression::<String>(engine, "PKG.license")?,
        url: url_option(engine, "PKG.url"),
        bug_url: url_option(engine, "PKG.bug_url"),
        bootstrap: from_expression::<bool>(engine, "PKG.bootstrap").unwrap_or(false),
        build_dependencies: vec![],
        check_dependencies: vec![],
        sources: vec![],
        packets: vec![],
    })
}
