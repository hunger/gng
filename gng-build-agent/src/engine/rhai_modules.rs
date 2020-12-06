// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Filesystem related rhai module:

use rhai::plugin::*;
use rhai::EvalAltResult;

use std::os::unix::fs::PermissionsExt;

// Define a module for filesystem-related tasks.
#[export_module]
mod fs_module {
    #[rhai_fn(return_raw)]
    pub fn chmod(
        mode: rhai::INT,
        path: &str,
    ) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        if mode > 0o7777 || mode < 0 {
            return Err(format!("Invalid mode 0o{:o} for {}.", mode, path).into());
        }
        let mode = mode as u32;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .map_err(|_| format!("Failed to change permissions on {}.", path))?;

        Ok(rhai::Dynamic::from(true))
    }

    #[rhai_fn(return_raw)]
    pub fn mkdir(directory: &str) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        std::fs::create_dir(directory)
            .map_err(|_| format!("Failed to create directory {}.", directory))?;
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
