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
use structopt::StructOpt;

use std::path::Path;
use std::sync::Arc;

use gng_build_shared::{cnt, env};

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

    let pkgsrc_dir = update_env(env::GNG_PKGSRC_DIR, cnt::GNG_PKGSRC_DIR.to_str().unwrap());
    let src_dir = update_env(env::GNG_SRC_DIR, cnt::GNG_SRC_DIR.to_str().unwrap());
    let inst_dir = update_env(env::GNG_INST_DIR, cnt::GNG_INST_DIR.to_str().unwrap());
    let pkg_dir = update_env(env::GNG_PKG_DIR, cnt::GNG_PKG_DIR.to_str().unwrap());

    let message_prefix =
        std::env::var(env::GNG_AGENT_MESSAGE_PREFIX).unwrap_or(String::from("MSG:"));
    std::env::remove_var(env::GNG_AGENT_MESSAGE_PREFIX);

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

    let build_rn = Path::new(&pkgsrc_dir).join("build.rn");
    sources.insert(runestick::Source::from_path(&build_rn).wrap_err(format!(
        "Failed to load \"{}\".",
        &build_rn.to_string_lossy()
    ))?);

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
