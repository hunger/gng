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

use eyre::eyre;
use structopt::StructOpt;

// - Helpers:
// ----------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(
    name = "gng-build-agent",
    about = "A package build agent for GnG.",
    rename_all = "kebab"
)]
enum Args {
    /// query package definition file
    QUERY,
    /// run the actual build process
    BUILD,
    /// Run tests and other checks
    CHECK,
    /// move the build results to their final location in the filesystem
    INSTALL,
    /// package the installed files
    PACKAGE,
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::trace!("Tracing subscriber initialized.");

    if !gng_shared::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    let args = Args::from_args();

    tracing::debug!("Command line arguments: {:#?}", args);

    println!("Hello, world!");

    Ok(())
}
