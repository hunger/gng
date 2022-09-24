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
#![allow(clippy::module_name_repetitions, clippy::let_unit_value)]

use std::path::PathBuf;

use clap::{builder::ValueParser, Parser};
use eyre::{Result, WrapErr};

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, Parser)]
#[clap(name = "gng-build", about = "A packet builder for GnG.")]
struct Args {
    /// configuration file to read
    #[clap(long, value_parser(ValueParser::os_string()), value_name = "FILE")]
    config: Option<PathBuf>,

    /// the repository to use
    #[clap(long, value_name = "REPO")]
    repository: Option<String>,

    /// The build agent to use
    #[clap(
        long,
        value_parser(ValueParser::os_string()),
        value_name = "EXECUTABLE",
        env = "GNG_AGENT_EXECUTABLE"
    )]
    agent: Option<PathBuf>,

    /// the directory containing the Lua run time environment
    #[clap(
        long,
        value_parser(ValueParser::os_string()),
        env = "GNG_LUA_DIR",
        value_name = "DIR"
    )]
    lua_dir: Option<PathBuf>,

    /// the directory to store temporary data
    #[clap(long, value_parser(ValueParser::os_string()), value_name = "DIR")]
    scratch_dir: Option<PathBuf>,

    /// the directory the build agent script will work in [DEBUG OPTION]
    #[clap(long, value_parser(ValueParser::os_string()), value_name = "DIR")]
    work_dir: Option<PathBuf>,

    /// the directory the build agent script will install into [DEBUG OPTION]
    #[clap(long, value_parser(ValueParser::os_string()), value_name = "DIR")]
    install_dir: Option<PathBuf>,

    /// The directory with the build information
    #[clap(value_parser(ValueParser::os_string()), value_name = "DIR")]
    recipes_dir: PathBuf,

    /// Keep temporary directories after build
    #[clap(long)]
    keep_temporaries: bool,

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

    let recipes_dir = std::env::current_dir()
        .wrap_err("Failed to get current work directory.")?
        .join(args.recipes_dir);

    let mut case_officer = gng_build::CaseOfficerBuilder::default();
    if let Some(tmp) = &args.lua_dir {
        case_officer.set_lua_directory(tmp);
    }
    if let Some(tmp) = &args.scratch_dir {
        case_officer.set_scratch_directory(tmp);
    }
    if let Some(tmp) = &args.agent {
        case_officer.set_agent(tmp);
    }

    let mut case_officer = case_officer
        .build(&recipes_dir)
        .wrap_err("Failed to initialize build container environment.")?;

    gng_build::handler::run(&mut case_officer)
}
