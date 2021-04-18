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
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

use gng_build_shared::constants::container as cc;
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
    /// move the build results to their final location in the filesystem
    Install,
    /// polish up the filesystem before putting all the files into a packet
    Polish,
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
    logging: gng_shared::log::LogArgs,
}

fn get_env(key: &str, default: &str) -> String {
    let result = std::env::var(key).unwrap_or_else(|_| default.to_owned());
    std::env::remove_var(key);
    result
}

fn get_message_prefix() -> String {
    let message_prefix =
        std::env::var(ce::GNG_AGENT_MESSAGE_PREFIX).unwrap_or_else(|_| String::from("MSG:"));
    std::env::remove_var(ce::GNG_AGENT_MESSAGE_PREFIX);

    message_prefix
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

struct Context {
    engine: gng_build_agent::engine::Engine,
}

fn prepare(ctx: &mut Context) -> Result<()> {
    ctx.engine
        .evaluate::<()>("prepare()")
        .wrap_err("Failed to prepare package")
}

fn build(ctx: &mut Context) -> Result<()> {
    ctx.engine
        .evaluate::<()>("build()")
        .wrap_err("Failed to build package.")
}

fn check(ctx: &mut Context) -> Result<()> {
    ctx.engine
        .evaluate::<()>("check()")
        .wrap_err("Failed to check package.")
}

fn install(ctx: &mut Context) -> Result<()> {
    ctx.engine
        .evaluate::<()>("install()")
        .wrap_err("Failed to install package.")
}

fn polish(ctx: &mut Context) -> Result<()> {
    ctx.engine
        .evaluate::<()>("polish()")
        .wrap_err("Failed to polish package.")
}

// ----------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

fn main() -> Result<()> {
    let args = Args::parse();

    args.logging
        .setup_logging()
        .wrap_err("Failed to set up logging.")?;

    if !gng_shared::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    let message_prefix = get_message_prefix();

    let mut engine_builder = gng_build_agent::engine::EngineBuilder::default();
    engine_builder.set_max_operations(4000)?;
    engine_builder.set_max_memory(4 * 1024 * 1024)?;

    engine_builder.push_string_constant(
        "WORK_DIR",
        std::fs::canonicalize(get_env(
            ce::GNG_WORK_DIR,
            cc::GNG_WORK_DIR.to_str().unwrap(),
        ))
        .wrap_err("Failed to turn WORK_DIR into canonical form")?
        .to_string_lossy()
        .as_ref(),
    )?;
    engine_builder.push_string_constant(
        "INST_DIR",
        std::fs::canonicalize(get_env(
            ce::GNG_INST_DIR,
            cc::GNG_INST_DIR.to_str().unwrap(),
        ))
        .wrap_err("Failed to turn INST_DIR into canonical form")?
        .to_string_lossy()
        .as_ref(),
    )?;

    let mut engine = engine_builder.eval_pkgsrc_directory()?;

    let source_packet = gng_build_agent::source_packet::from_engine(&mut engine)?;

    tracing::trace!(
        "Read {} file for {}",
        gng_build_shared::BUILD_SCRIPT,
        source_packet
    );

    send_message(
        &message_prefix,
        &gng_build_shared::MessageType::Data,
        &serde_json::to_string(&source_packet)?,
    );

    let mut ctx = Context { engine };

    match args.subcommand {
        SubCommand::Query => Ok(()),
        SubCommand::Prepare => prepare(&mut ctx),
        SubCommand::Build => build(&mut ctx),
        SubCommand::Check => check(&mut ctx),
        SubCommand::Install => install(&mut ctx),
        SubCommand::Polish => polish(&mut ctx),
    }
}
