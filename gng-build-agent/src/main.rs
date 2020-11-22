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

use eyre::{eyre, WrapErr};
use runestick::FromValue;
use structopt::StructOpt;

use std::path::Path;
use std::sync::Arc;

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
    /// run the actual build process
    BUILD,
    /// Run tests and other checks
    CHECK,
    /// move the build results to their final location in the filesystem
    INSTALL,
    /// package the installed files
    PACKAGE,
}

fn update_env(key: &str, default: &str) -> String {
    let result = std::env::var(key).unwrap_or(default.to_owned());
    std::env::set_var(key, &result);
    result
}

fn ls(path: &Path) {
    let paths = std::fs::read_dir(&path).unwrap();

    println!("Contents of {:?}:", &path);
    for path in paths {
        println!("    {}", path.unwrap().path().display())
    }
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::trace!("Tracing subscriber initialized.");

    if !gng_shared::is_root() {
        return Err(eyre!("This application needs to be run by root."));
    }

    let args = Args::from_args();

    tracing::debug!("Command line arguments: {:#?}", args);

    let pkgsrc_dir = update_env("GNG_PKGSRC_DIR", "/gng/pkgsrc");
    let src_dir = update_env("GNG_SRC_DIR", "/gng/src");
    let inst_dir = update_env("GNG_INST_DIR", "/gng/inst");
    let pkg_dir = update_env("GNG_PKG_DIR", "/gng/pkg");

    for p in vec![
        Path::new("/"),
        Path::new("/etc"),
        Path::new("/usr"),
        Path::new("/gng"),
        Path::new("/gng/pkgsrc"),
    ]
    .into_iter()
    {
        ls(p);
    }

    let context = runestick::Context::with_default_modules()?;
    let mut sources = rune::Sources::new();

    sources.insert(
        runestick::Source::from_path(&Path::new(&pkgsrc_dir).join("build.rune"))
            .wrap_err(format!("Failed to load \"{}/build.rune\".", &pkgsrc_dir))?,
    );

    let mut errors = rune::Errors::new();
    let mut warnings = rune::Warnings::new();

    let unit = rune::load_sources(
        &context,
        &rune::Options::default(),
        &mut sources,
        &mut errors,
        &mut warnings,
    )?;

    let vm = runestick::Vm::new(Arc::new(context), Arc::new(unit));
    let output = vm.execute(&["main"], ())?.complete()?;

    Ok(())
}
