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

use std::{convert::TryFrom, path::PathBuf, str::FromStr};

use clap::Clap;
use eyre::{eyre, Result, WrapErr};

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, clap::Clap)]
#[clap(name = "gng-build", about = "A packet builder for GnG.")]
struct Args {
    /// configuration file to read
    #[clap(long, value_name = "PACKET_NAME")]
    packet_name: String,

    /// configuration file to read
    #[clap(long, value_name = "PACKET_VERSION")]
    packet_version: String,

    /// the directory to package up
    #[clap(long, parse(from_os_str), value_name = "DIR")]
    package_dir: PathBuf,

    /// the directory to store temporary data
    #[clap(long, default_value = "", value_name = "GLOB PATTERNS")]
    globs: String,

    #[clap(flatten)]
    logging: gng_shared::log::LogArgs,
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

/// Entry point of the `gng-package` binary.
fn main() -> Result<()> {
    let args = Args::parse();

    let _app_span = args
        .logging
        .setup_logging()
        .wrap_err("Failed to set up logging.")?;

    if !gng_shared::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    tracing::debug!("Command line arguments: {:#?}", args);

    let globs = args
        .globs
        .split(&":")
        .map(|s| glob::Pattern::from_str(s))
        .collect::<Result<Vec<glob::Pattern>, glob::PatternError>>()
        .map_err(|e| eyre!("Invalid glob pattern given on command line: {}", e))?;

    let p = gng_shared::PacketBuilder::default()
        .source_name(gng_shared::Name::try_from("manual")?)
        .license("unknown".to_string())
        .name(gng_shared::Name::try_from(args.packet_name.as_str())?)
        .version(gng_shared::Version::try_from(args.packet_version.as_str())?)
        .description("unknown".to_string())
        .build()
        .map_err(|e| gng_shared::Error::Runtime {
            message: format!("Failed to define a packet: {}", e),
        })?;

    let mut packager = gng_build::PackagerBuilder::default()
        .add_packet(&p, &globs)?
        .build()?;

    let package_files = packager
        .package(&args.package_dir, &std::env::current_dir()?)
        .wrap_err(format!(
            "Failed to package \"{}\".",
            args.package_dir.to_string_lossy()
        ))?;

    for pf in package_files {
        println!("Package \"{}\" created.", pf.to_string_lossy());
    }

    Ok(())
}
