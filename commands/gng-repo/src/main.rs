// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The `gng-repo` binary.

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
#![allow(clippy::module_name_repetitions, clippy::let_unit_value)]

use std::path::PathBuf;

use clap::Clap;
use eyre::{Result, WrapErr};

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
#[clap(name = "gng-repo", about = "A repository manager for GnG.")]
struct Args {
    /// configuration file to read
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    config: Option<PathBuf>,

    /// Start from scratch: It is OK if there is no `repository.json` file
    #[clap(long)]
    from_scratch: bool,

    /// Clear all existing data from `repository.json`
    #[clap(long)]
    clear: bool,

    /// the repository to use
    #[clap(parse(from_os_str), value_name = "REPO_DIR")]
    repository_directory: PathBuf,

    /// the packets to add to the repository
    #[clap(parse(from_os_str), value_name = "GNG_FILE")]
    packets: Vec<PathBuf>,

    #[clap(flatten)]
    logging: gng_core::log::LogArgs,
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

/// Entry point of the `gng-build` binary.
fn main() -> Result<()> {
    let args = Args::parse();

    let _app_span = args
        .logging
        .setup_logging()
        .wrap_err("Failed to set up logging.")?;

    tracing::debug!("Command line arguments: {:#?}", args);

    if args.packets.is_empty() {
        tracing::warn!("No packets provided, nothing to do.");
        return Ok(());
    }

    let mut repo = gng_packet_db::Repository::from_local_directory(
        &args.repository_directory,
        args.from_scratch,
    )?;

    let mut update = repo.create_transaction();
    if args.clear {
        update.clear();
    }

    for p in &args.packets {
        update.add_packet_file(p)?;
    }

    repo.apply(update)?;

    repo.save_local_directory()
}
