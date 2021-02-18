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

use eyre::{eyre, Result, WrapErr};
use structopt::StructOpt;

// - Helpers:
// ----------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(
    name = "gng-build-agent",
    about = "A packet build agent for GnG.",
    rename_all = "kebab"
)]
enum Args {
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

fn prepare(_ctx: &mut Context) -> Result<()> {
    Ok(())
}

fn build(_ctx: &mut Context) -> Result<()> {
    Ok(())
}

fn check(_ctx: &mut Context) -> Result<()> {
    Ok(())
}

fn install(_ctx: &mut Context) -> Result<()> {
    Ok(())
}

fn polish(_ctx: &mut Context) -> Result<()> {
    Ok(())
}

// ----------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init()
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to set up tracing")?;
    tracing::trace!("Tracing subscriber initialized.");

    if !gng_shared::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    let args = Args::from_args();
    tracing::trace!("Command line arguments: {:#?}", args);

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
        &gng_build_shared::MessageType::DATA,
        &serde_json::to_string(&source_packet)?,
    );

    let mut ctx = Context { engine };

    match args {
        Args::Query => Ok(()),
        Args::Prepare => prepare(&mut ctx),
        Args::Build => build(&mut ctx),
        Args::Check => check(&mut ctx),
        Args::Install => install(&mut ctx),
        Args::Polish => polish(&mut ctx),
    }
}
