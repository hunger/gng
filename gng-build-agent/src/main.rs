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

use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;
use gng_build_shared::SourcePacket;

use structopt::StructOpt;

use std::path::Path;

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
    QUERY,
    /// prepare the sources for the build
    PREPARE,
    /// run the actual build process
    BUILD,
    /// Run tests and other checks
    CHECK,
    /// move the build results to their final location in the filesystem
    INSTALL,
    /// package the installed files into `Packet`s
    PACKAGE,
}

fn get_env(key: &str, default: &str) -> String {
    let result = std::env::var(key).unwrap_or(default.to_owned());
    std::env::remove_var(key);
    result
}

fn get_message_prefix() -> String {
    let message_prefix =
        std::env::var(ce::GNG_AGENT_MESSAGE_PREFIX).unwrap_or(String::from("MSG:"));
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

struct Context<'a> {
    engine: gng_build_agent::engine::Engine<'a>,
    source_packet: SourcePacket,
    message_prefix: String,
}

fn query(ctx: &mut Context) -> eyre::Result<()> {
    Ok(())
}

fn prepare(ctx: &mut Context) -> eyre::Result<()> {
    todo!();
}

fn build(ctx: &mut Context) -> eyre::Result<()> {
    todo!();
}

fn check(ctx: &mut Context) -> eyre::Result<()> {
    todo!();
}

fn install(ctx: &mut Context) -> eyre::Result<()> {
    todo!();
}

fn package(ctx: &mut Context) -> eyre::Result<()> {
    todo!();
}

// ----------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::trace!("Tracing subscriber initialized.");

    if !gng_shared::is_root() {
        return Err(eyre::eyre!("This application needs to be run by root."));
    }

    let args = Args::from_args();
    tracing::trace!("Command line arguments: {:#?}", args);

    let message_prefix = get_message_prefix();

    let pkgsrc_dir = get_env(ce::GNG_PKGSRC_DIR, cc::GNG_PKGSRC_DIR.to_str().unwrap());

    let mut engine_builder = gng_build_agent::engine::EngineBuilder::default();
    engine_builder.push_constant("PKGSRC_DIR", pkgsrc_dir.clone().into());
    engine_builder.push_constant(
        "SRC_DIR",
        get_env(ce::GNG_PKGSRC_DIR, cc::GNG_PKGSRC_DIR.to_str().unwrap()).into(),
    );
    engine_builder.push_constant(
        "INST_DIR",
        get_env(ce::GNG_PKGSRC_DIR, cc::GNG_INST_DIR.to_str().unwrap()).into(),
    );
    engine_builder.push_constant(
        "PKG_DIR",
        get_env(ce::GNG_PKGSRC_DIR, cc::GNG_PKG_DIR.to_str().unwrap()).into(),
    );

    let mut engine = engine_builder.eval_pkgsrc_directory(&Path::new(&pkgsrc_dir))?;

    let source_packet = gng_build_agent::source_packet::from_engine(&mut engine)?;

    tracing::trace!("Read build.rhai file for {}", source_packet);

    send_message(
        &message_prefix,
        &gng_build_shared::MessageType::DATA,
        &serde_json::to_string(&source_packet)?,
    );

    let mut ctx = Context {
        engine,
        source_packet,
        message_prefix,
    };

    match args {
        Args::QUERY => query(&mut ctx),
        Args::PREPARE => prepare(&mut ctx),
        Args::BUILD => build(&mut ctx),
        Args::CHECK => check(&mut ctx),
        Args::INSTALL => install(&mut ctx),
        Args::PACKAGE => package(&mut ctx),
    }
}
