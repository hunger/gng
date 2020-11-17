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

use eyre::Result;
use gumdrop::Options;

#[derive(Debug, Options)]
struct Args {
    #[options(help = "print help message")]
    help: bool,

    #[options(help = "config file to read", meta = "<FILE>")]
    config: Option<String>,

    #[options(command)]
    command: Option<Command>,
}

/// Define the commands understood by `gng-build`.
#[derive(Debug, Options)]
enum Command {
    #[options(help = "show help for a command")]
    Help(HelpArgs),
}

/// Command line arguments for the `help` command.
#[derive(Debug, Options)]
struct HelpArgs {
    #[options(free, help = "further arguments")]
    args: Vec<String>,
}

/// Entry point of the `gng-build` binary.
fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::trace!("Tracing subscriber initialized.");

    let args = Args::parse_args_default_or_exit();

    println!("{:#?}", args);

    Ok(())
}
