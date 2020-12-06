// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Filesystem related rhai module:

// #![allow(clippy::wildcard_imports)] // rhai depends on all symbols being available
use rhai::plugin::{
    export_module, mem, new_vec, CallableFunction, FnAccess, FnNamespace, Module,
    NativeCallContext, PluginFunction, TypeId,
};
use rhai::{Dynamic, EvalAltResult, ImmutableString};

use std::convert::TryFrom;
use std::os::unix::fs::PermissionsExt;

// Define a module for filesystem-related tasks.
#[export_module]
mod fs_module {
    #[rhai_fn(return_raw)]
    pub fn chmod(
        mode: rhai::INT,
        path: &str,
    ) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        if !(0..=0o7777).contains(&mode) {
            return Err(format!("Invalid mode 0o{:o} for {}.", mode, path).into());
        }
        let mode = u32::try_from(mode).expect("Was in a safe range just now!");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| {
            format!(
                "Failed to change permissions on {}: {}",
                path,
                e.to_string()
            )
        })?;

        Ok(rhai::Dynamic::from(true))
    }

    #[rhai_fn(return_raw)]
    pub fn mkdir(directory: &str) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        std::fs::create_dir(directory).map_err(|e| {
            format!(
                "Failed to create directory {}: {}",
                directory,
                e.to_string()
            )
        })?;
        Ok(rhai::Dynamic::from(true))
    }

    #[rhai_fn(name = "mkdir", return_raw)]
    pub fn mkdir_mode(
        mode: rhai::INT,
        directory: &str,
    ) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        mkdir(directory)?;
        chmod(mode, directory)
    }
}
