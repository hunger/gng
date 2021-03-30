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

fn converted_expression<'a, T: std::convert::TryFrom<String, Error = gng_shared::Error> + Clone>(
    engine: &'a mut crate::engine::Engine,
    expression: &'a str,
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

fn extract_array<T>(
    engine: &mut crate::engine::Engine,
    base_expression: &str,
    converter: impl Fn(&mut crate::engine::Engine, &str) -> gng_shared::Result<T>,
) -> gng_shared::Result<Vec<T>> {
    let element_count = from_expression::<usize>(engine, &format!("#{}", base_expression))?;
    let mut result = Vec::with_capacity(element_count);

    for count in 1..=element_count {
        result.push(converter(
            engine,
            &format!("{}[{}]", base_expression, count),
        )?);
    }

    Ok(result)
}

fn extract_facet(
    engine: &mut crate::engine::Engine,
    expr: &str,
) -> gng_shared::Result<Option<gng_build_shared::Facet>> {
    println!("Extracting Facet data.");
    if from_expression::<bool>(
        engine,
        &format!("{} == nil or next({}) == nil", expr, expr,),
    )? {
        println!("Extracting Facet data: NONE");
        Ok(None)
    } else {
        println!("Extracting Facet data: Some");
        Ok(Some(gng_build_shared::Facet {
            description_suffix: from_expression::<String>(
                engine,
                &format!("{}.description_suffix or \"\"", expr,),
            )?,
            mime_types: extract_array(engine, &format!("{}.mime_types", expr), |engine, expr| {
                from_expression::<String>(engine, expr)
            })?,
            patterns: extract_array(engine, &format!("{}.patterns", expr), |engine, expr| {
                from_expression::<String>(engine, expr)
            })?,
        }))
    }
}

fn extract_packets(
    engine: &mut crate::engine::Engine,
    packet_base: &str,
) -> gng_shared::Result<Vec<gng_build_shared::PacketDefinition>> {
    extract_array(engine, packet_base, |engine, expr| {
        println!("extracting one packet");

        Ok(gng_build_shared::PacketDefinition {
            name: converted_expression::<Name>(engine, &format!("{}.name", expr))?,
            description: from_expression::<String>(engine, &format!("{}.description", expr))?,
            dependencies: extract_array(
                engine,
                &format!("{}.dependencies", expr),
                |engine, expr| converted_expression::<Name>(engine, expr),
            )?,
            files: extract_array(engine, &format!("{}.files", expr), |engine, expr| {
                from_expression::<String>(engine, expr)
            })?,
            facet: extract_facet(engine, &format!("{}.facet", expr))?,
        })
    })
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
        packets: extract_packets(engine, "PKG.packets")?,
    })
}
