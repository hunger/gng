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

use gng_build_agent::source_package::SourcePackage;
use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;

use structopt::StructOpt;

use std::path::Path;

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
    /// prepare the sources for the build
    PREPARE,
    /// run the actual build process
    BUILD,
    /// Run tests and other checks
    CHECK,
    /// move the build results to their final location in the filesystem
    INSTALL,
    /// package the installed files
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
fn query(_source_package: &SourcePackage, _message_prefix: &str) -> eyre::Result<()> {
    Ok(())
}

fn prepare(_source_package: &SourcePackage, _message_prefix: &str) -> eyre::Result<()> {
    todo!();
}

fn build(_source_package: &SourcePackage, _message_prefix: &str) -> eyre::Result<()> {
    todo!();
}

fn check(_source_package: &SourcePackage, _message_prefix: &str) -> eyre::Result<()> {
    todo!();
}

fn install(_source_package: &SourcePackage, _message_prefix: &str) -> eyre::Result<()> {
    todo!();
}

fn package(_source_package: &SourcePackage, _message_prefix: &str) -> eyre::Result<()> {
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

    let source_package = gng_build_agent::source_package::SourcePackage::new(
        engine_builder.eval_pkgsrc_directory(&Path::new(&pkgsrc_dir))?,
    )?;

    tracing::trace!("Read build.rhai file for {}", source_package);

    send_message(
        &message_prefix,
        &gng_build_shared::MessageType::DATA,
        &serde_json::to_string(&source_package)?,
    );

    match args {
        Args::QUERY => query(&source_package, &message_prefix),
        Args::PREPARE => prepare(&source_package, &message_prefix),
        Args::BUILD => build(&source_package, &message_prefix),
        Args::CHECK => check(&source_package, &message_prefix),
        Args::INSTALL => install(&source_package, &message_prefix),
        Args::PACKAGE => package(&source_package, &message_prefix),
    }
}
