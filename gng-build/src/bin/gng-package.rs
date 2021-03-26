// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The `gng-package` binary.
//!
//! This is a very simplistic tarball generator.

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

use std::{path::PathBuf, str::FromStr};

use eyre::{eyre, Result, WrapErr};
use structopt::StructOpt;

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(name = "gng-build", about = "A packet builder for GnG.")]
struct Args {
    /// configuration file to read
    #[structopt(long, value_name = "FILE_NAME")]
    packet_name: String,

    /// the directory containing the Lua runtime environment
    #[structopt(long, parse(from_os_str), value_name = "DIR")]
    packet_dir: PathBuf,

    /// the directory to store temporary data
    #[structopt(long, default_value = "", value_name = "GLOB PATTERNS")]
    globs: String,
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

/// Entry point of the `gng-package` binary.
fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init()
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to set up tracing")?;
    tracing::trace!("Tracing subscriber initialized.");

    if !gng_shared::is_root() {
        // TODO: Enable this!
        // return Err(eyre!("This application needs to be run by root."));
    }

    let args = Args::from_args();

    tracing::debug!("Command line arguments: {:#?}", args);

    let globs = args
        .globs
        .split(&":")
        .map(|s| glob::Pattern::from_str(s))
        .collect::<Result<Vec<glob::Pattern>, glob::PatternError>>()
        .map_err(|e| eyre!("Invalid glob pattern given on command line: {}", e))?;

    let p = gng_shared::PacketBuilder::default()
        .try_source_name("manual")?
        .license("unknown")
        .try_version("unknown")?
        .try_name(args.packet_name.as_str())?
        .description("Unknown")
        .build()
        .map_err(|e| gng_shared::Error::Runtime {
            message: format!("Failed to define a packet: {}", e),
        })?;

    let mut packager = gng_build::PackagerBuilder::default()
        .add_packet(&p, &globs, &[])?
        .build();

    let package_files = packager.package(&args.packet_dir).wrap_err(format!(
        "Failed to package \"{}\".",
        args.packet_dir.to_string_lossy()
    ))?;

    for pf in package_files {
        println!("Package \"{}\" created.", pf.to_string_lossy());
    }

    Ok(())
}
