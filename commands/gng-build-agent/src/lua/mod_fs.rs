// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The `fs` module for Lua

// spell-checker: ignore chdir currentdir mkdir

#![allow(clippy::needless_pass_by_value)] // Lua can not pass &str!
#![allow(dead_code)] // Dead code is called from Lua

use eyre::WrapErr;
use rlua::ToLuaMulti;

// ----------------------------------------------------------------------
// - Lua Module `fs`:
// ----------------------------------------------------------------------

fn chdir(
    lua_context: rlua::Context,
    path: String,
) -> std::result::Result<rlua::MultiValue, rlua::Error> {
    match std::env::set_current_dir(&path) {
        Err(e) => (
            rlua::Value::Nil,
            lua_context.pack(format!("Failed to change directory to \"{}\": {}", path, e))?,
        )
            .to_lua_multi(lua_context),
        Ok(()) => (lua_context.pack(true)?, rlua::Value::Nil).to_lua_multi(lua_context),
    }
}

fn current_dir(
    lua_context: rlua::Context,
    _: (),
) -> std::result::Result<rlua::MultiValue, rlua::Error> {
    match std::env::current_dir() {
        Err(e) => (
            rlua::Value::Nil,
            lua_context.pack(format!("Current directory not found: {}", e))?,
        )
            .to_lua_multi(lua_context),
        Ok(d) => (
            lua_context.pack(d.to_string_lossy().as_ref().to_owned())?,
            rlua::Value::Nil,
        )
            .to_lua_multi(lua_context),
    }
}

fn mkdir(
    lua_context: rlua::Context,
    path: String,
) -> std::result::Result<rlua::MultiValue, rlua::Error> {
    match std::fs::create_dir(path) {
        Err(e) => (
            rlua::Value::Nil,
            lua_context.pack(format!("Can not create directory: {}", e))?,
            lua_context.pack(1)?,
        )
            .to_lua_multi(lua_context),
        Ok(()) => {
            (lua_context.pack(true)?, rlua::Value::Nil, rlua::Value::Nil).to_lua_multi(lua_context)
        }
    }
}

fn rmdir(
    lua_context: rlua::Context,
    path: String,
) -> std::result::Result<rlua::MultiValue, rlua::Error> {
    match std::fs::remove_dir(path) {
        Err(e) => (
            rlua::Value::Nil,
            lua_context.pack(format!("Can not remove directory: {}", e))?,
            lua_context.pack(1)?,
        )
            .to_lua_multi(lua_context),
        Ok(()) => {
            (lua_context.pack(true)?, rlua::Value::Nil, rlua::Value::Nil).to_lua_multi(lua_context)
        }
    }
}

// - Register:
// ----------------------------------------------------------------------

/// Register the `fs` module with Lua
#[tracing::instrument(level = "trace", skip(lua))]
pub fn register(lua: &mut rlua::Lua) -> eyre::Result<()> {
    lua.context(|lua_context| -> std::result::Result<(), rlua::Error> {
        let fn_table = lua_context.create_table()?;

        fn_table.set("chdir", lua_context.create_function(chdir)?)?;
        fn_table.set("currentdir", lua_context.create_function(current_dir)?)?;
        fn_table.set("mkdir", lua_context.create_function(mkdir)?)?;
        fn_table.set("rmdir", lua_context.create_function(rmdir)?)?;

        // Set up Lua side:
        lua_context.globals().set("fs", fn_table)?;

        Ok(())
    })
    .wrap_err("Export of `fs` module functions into Lua runtime failed.")
}
