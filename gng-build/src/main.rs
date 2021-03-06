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

use gng_db::GngDbExt;

use std::path::PathBuf;

use clap::Clap;
use eyre::{Result, WrapErr};

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
#[clap(name = "gng-build", about = "A packet builder for GnG.")]
struct Args {
    /// configuration file to read
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    config: Option<PathBuf>,

    /// the repository to use
    #[clap(long, value_name = "REPO")]
    repository: Option<String>,

    /// The build agent to use
    #[clap(
        long,
        parse(from_os_str),
        value_name = "EXECUTABLE",
        env = "GNG_AGENT_EXECUTABLE"
    )]
    agent: Option<PathBuf>,

    /// the directory containing the Lua runtime environment
    #[clap(long, parse(from_os_str), env = "GNG_LUA_DIR", value_name = "DIR")]
    lua_dir: Option<PathBuf>,

    /// the directory to store temporary data
    #[clap(long, parse(from_os_str), value_name = "DIR")]
    scratch_dir: Option<PathBuf>,

    /// the directory the build agent script will work in [DEBUG OPTION]
    #[clap(long, parse(from_os_str), value_name = "DIR")]
    work_dir: Option<PathBuf>,

    /// the directory the build agent script will install into [DEBUG OPTION]
    #[clap(long, parse(from_os_str), value_name = "DIR")]
    install_dir: Option<PathBuf>,

    /// The directory with the build information
    #[clap(parse(from_os_str), value_name = "DIR")]
    pkgsrc_dir: PathBuf,

    /// Keep temporary directories after build
    #[clap(long)]
    keep_temporaries: bool,

    #[clap(flatten)]
    logging: gng_shared::log::LogArgs,

    #[clap(flatten)]
    gng: gng_shared::gng::GngArgs,
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

    let config = args.gng.create_gng()?;

    tracing::debug!("Command line arguments: {:#?}", args);

    let pkgsrc_dir = std::env::current_dir()
        .wrap_err("Failed to get current work directory.")?
        .join(args.pkgsrc_dir);

    let repo_db = config.repository_db()?;

    let repo = match &args.repository {
        Some(rin) => repo_db.resolve_repository(rin),
        None => repo_db.repository_for_packet_source_path(&pkgsrc_dir),
    }
    .ok_or_else(|| eyre::eyre!("Could not find repository to adopt the build result into."))?;
    tracing::debug!("Inserting packets into repository {}.", repo.name);

    let source_packet_info = std::rc::Rc::new(gng_build::handler::SourcePacketInfo::default());

    let mut case_officer = gng_build::CaseOfficerBuilder::default();
    if let Some(tmp) = &args.lua_dir {
        case_officer.set_lua_directory(tmp);
    }
    if let Some(tmp) = &args.scratch_dir {
        case_officer.set_scratch_directory(tmp);
    }
    if let Some(tmp) = &args.work_dir {
        case_officer.set_work_directory(tmp);
    }
    if let Some(tmp) = &args.install_dir {
        case_officer.set_install_directory(tmp);
    }
    if let Some(tmp) = &args.agent {
        case_officer.set_agent(tmp);
    }

    let mut case_officer = case_officer
        .add_handler(Box::new(
            gng_build::handler::ImmutableSourceDataHandler::default(),
        ))
        .add_handler(Box::new(gng_build::handler::ParseSourceDataHandler::new(
            source_packet_info.clone(),
        )))
        .add_handler(Box::new(gng_build::handler::ValidateHandler::new(
            source_packet_info.clone(),
        )))
        .add_handler(Box::new(gng_build::handler::PackagingHandler::new(
            source_packet_info,
        )))
        .build(&pkgsrc_dir)
        .wrap_err("Failed to initialize build container environment.")?;

    case_officer.process()?;

    Ok(())
}
