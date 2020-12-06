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

use std::path::PathBuf;

use eyre::{eyre, WrapErr};
use structopt::StructOpt;

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(name = "gng-build", about = "A packet builder for GnG.")]
struct Args {
    /// configuration file to read
    #[structopt(long, parse(from_os_str), value_name = "FILE")]
    config: Option<PathBuf>,

    /// the directory to store temporary data
    #[structopt(long, parse(from_os_str), value_name = "DIR")]
    scratch_dir: Option<PathBuf>,

    /// the directory the build agent script will work in [DEBUG OPTION]
    #[structopt(long, parse(from_os_str), value_name = "DIR")]
    work_dir: Option<PathBuf>,

    /// the directory the build agent script will install into [DEBUG OPTION]
    #[structopt(long, parse(from_os_str), value_name = "DIR")]
    install_dir: Option<PathBuf>,

    /// The build agent to use
    #[structopt(long, parse(from_os_str), value_name = "EXECUTABLE")]
    agent: Option<PathBuf>,

    /// The directory with the build information
    #[structopt(parse(from_os_str), value_name = "DIR")]
    pkgsrc_dir: PathBuf,

    /// Keep temporary directories after build
    #[structopt(long)]
    keep_temporaries: bool,
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

/// Entry point of the `gng-build` binary.
fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::try_init()
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to set up logging")?;
    tracing::trace!("Tracing subscriber initialized.");

    if !gng_shared::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    let args = Args::from_args();

    tracing::debug!("Command line arguments: {:#?}", args);

    let mut case_officer = gng_build::CaseOfficerBuilder::default();
    if args.scratch_dir.is_some() {
        case_officer.set_scratch_directory(&args.scratch_dir.unwrap());
    }
    if args.work_dir.is_some() {
        case_officer.set_work_directory(&args.work_dir.unwrap());
    }
    if args.install_dir.is_some() {
        case_officer.set_install_directory(&args.install_dir.unwrap());
    }

    let mut case_officer = case_officer
        .set_agent(&args.agent.unwrap())
        .add_message_handler(Box::new(
            gng_build::message_handler::ImmutableSourceDataHandler::default(),
        ))
        .build(&args.pkgsrc_dir)
        .wrap_err("Failed to initialize build container environment.")?;

    case_officer.process()?;

    Ok(())
}
