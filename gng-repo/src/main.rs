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
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

use std::path::PathBuf;

use clap::Clap;
use eyre::{Result, WrapErr};

use gng_repository::repository::Repository;

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
#[clap(name = "gng-repo", about = "A repository query tool for GnG.")]
struct Args {
    /// the directory containing the Lua runtime environment
    #[clap(
        long = "repository",
        parse(from_os_str),
        env = "GNG_REPOSITORY",
        value_name = "DIR"
    )]
    repository_dir: PathBuf,

    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    logging: gng_shared::log::LogArgs,
}

#[derive(Debug, Clap)]
enum Commands {
    Internal(InternalCommands),
}

// ----------------------------------------------------------------------
// - InternalCommands:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
struct InternalCommands {
    #[clap(subcommand)]
    sub_command: InternalSubCommands,
}

#[derive(Debug, Clap)]
enum InternalSubCommands {
    Metadata,
}

fn handle_internal_command(repo: &mut impl Repository, cmd: &InternalCommands) -> Result<()> {
    match cmd.sub_command {
        InternalSubCommands::Metadata => repo
            .dump_metadata()
            .wrap_err("Repository storage backend failed to dump meta data."),
    }
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

    let mut repo = gng_repository::open(&args.repository_dir).wrap_err(format!(
        "Failed to open repository at \"{}\".",
        args.repository_dir.to_string_lossy()
    ))?;

    match args.command {
        Commands::Internal(cmd) => handle_internal_command(&mut repo, &cmd),
    }
}
