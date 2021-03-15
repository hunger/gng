// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The `gng-build` binary.

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

use std::path::PathBuf;

use eyre::{eyre, Result, WrapErr};
use structopt::StructOpt;

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(name = "gng-build", about = "A packet builder for GnG.")]
struct Args {
    /// configuration file to read
    #[structopt(long, parse(from_os_str), value_name = "FILE")]
    packet_name: PathBuf,

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

    if !args.packet_dir.is_dir() {
        return Err(eyre!(format!(
            "\"paket_dir\" {} is not a directory.",
            args.packet_dir.to_string_lossy()
        )));
    }

    Ok(())
}
