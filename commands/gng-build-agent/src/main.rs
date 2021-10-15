// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The `gng-build-agent` binary.

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

use gng_build_agent::script_support::ScriptSupport;
use gng_build_shared::constants::environment as ce;

use clap::Clap;
use eyre::{eyre, Result, WrapErr};

// - Helpers:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
enum SubCommand {
    /// query packet definition file
    Query,
    /// prepare the sources for the build
    Prepare,
    /// run the actual build process
    Build,
    /// Run tests and other checks
    Check,
    /// move the build results to their final location in the file system
    Install,
    /// polish up the file system before putting all the files into a packet
    Polish,
    /// package up the file system [NOOP on agent side]
    Package,
}

impl std::str::FromStr for SubCommand {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_ref() {
            "query" => Ok(Self::Query),
            "prepare" => Ok(Self::Prepare),
            "build" => Ok(Self::Build),
            "check" => Ok(Self::Check),
            "install" => Ok(Self::Install),
            "polish" => Ok(Self::Polish),
            "package" => Ok(Self::Package),
            _ => Err(eyre::eyre!("Invalid subcommand given")),
        }
    }
}

#[derive(Debug, Clap)]
#[clap(
    name = "gng-build-agent",
    about = "A packet build agent for GnG.",
    rename_all = "kebab"
)]
struct Args {
    subcommand: SubCommand,
    #[clap(flatten)]
    logging: gng_core::log::LogArgs,
}

fn get_message_prefix() -> String {
    gng_build_agent::take_env(ce::GNG_AGENT_MESSAGE_PREFIX, "unknown")
}

fn send_message(message_prefix: &str, message_type: &gng_build_shared::MessageType, message: &str) {
    println!(
        "MSG_{}_{}: {}",
        message_prefix,
        String::from(message_type),
        message
    );
}

// ----------------------------------------------------------------------
// - Commands:
// ----------------------------------------------------------------------

fn run_subcommand(script_support: &mut impl ScriptSupport, subcommand: &SubCommand) -> Result<()> {
    match subcommand {
        SubCommand::Query | SubCommand::Package => Ok(()),
        SubCommand::Prepare => script_support.prepare(),
        SubCommand::Build => script_support.build(),
        SubCommand::Check => script_support.check(),
        SubCommand::Install => script_support.install(),
        SubCommand::Polish => script_support.polish(),
    }
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

fn main() -> Result<()> {
    let args = Args::parse();

    let _app_span = args
        .logging
        .setup_logging()
        .wrap_err("Failed to set up logging.")?;

    if !gng_core::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    let message_prefix = get_message_prefix();

    let mut script_support = gng_build_agent::create_script_support()?;

    let source_packet = script_support
        .parse_build_script()
        .wrap_err("Failed to parse the build script.")?;

    send_message(
        &message_prefix,
        &gng_build_shared::MessageType::Data,
        &serde_json::to_string(&source_packet)?,
    );

    run_subcommand(&mut script_support, &args.subcommand)
}
