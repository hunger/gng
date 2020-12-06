// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Filesystem related rhai module:

use rhai::plugin::*;

// Define plugin module.
#[export_module]
mod fs_module {
    pub fn greet(name: &str) -> String {
        format!("hello, {}!", name)
    }
}
