// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! `gng-build-agent` functionality

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

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

/// Read an environment variable and remove it from the environment.
#[must_use]
pub fn take_env(key: &str, default: &str) -> String {
    let result = std::env::var(key).unwrap_or_else(|_| default.to_owned());
    std::env::remove_var(key);
    result
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

pub mod lua;
pub mod script_support;

/// Create a new instance of `ScriptSupport`
///
/// # Errors
/// Return some error if setting up the environment fails
pub fn create_script_support() -> eyre::Result<impl crate::script_support::ScriptSupport> {
    crate::lua::LuaScriptSupport::new()
}
