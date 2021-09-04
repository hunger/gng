// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Lua support for `gng-build-agent`

pub mod mod_fs;

use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;

use eyre::WrapErr;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn map_error(error: &rlua::Error) -> gng_core::Error {
    gng_core::Error::Script {
        message: error.to_string(),
    }
}

fn set_max_operations(lua: &mut rlua::Lua, count: u32) {
    lua.set_hook(
        rlua::HookTriggers {
            every_nth_instruction: Some(count),
            ..rlua::HookTriggers::default()
        },
        |_lua_context, _debug| {
            Err(rlua::Error::RuntimeError(
                "Too many operations!".to_string(),
            ))
        },
    );
}

fn set_max_memory(lua: &mut rlua::Lua, size: usize) {
    lua.set_memory_limit(Some(size));
}

fn push_string_constant(lua: &mut rlua::Lua, key: &str, value: &str) -> eyre::Result<()> {
    lua.context(|lua_ctx| lua_ctx.globals().set(key, value).map_err(|e| map_error(&e)))?;
    Ok(())
}

fn setup_lua(lua: &mut rlua::Lua) -> eyre::Result<()> {
    mod_fs::register(lua)
}

fn evaluate<T: serde::de::DeserializeOwned>(
    lua: &mut rlua::Lua,
    expression: &str,
) -> eyre::Result<T> {
    lua.context(|lua_context| -> eyre::Result<T> {
        let value = lua_context
            .load(expression)
            .eval::<rlua::Value>()
            .wrap_err(format!(
                "Failed to evaluate \"{}\" in Lua engine.",
                expression
            ))?;
        rlua_serde::from_value(value).wrap_err(format!(
            "Failed to convert \"{}\" from Lua to Rust.",
            expression
        ))
    })
}

fn eval_pkgsrc_directory(lua: &mut rlua::Lua) -> eyre::Result<()> {
    let build_file = std::path::PathBuf::from(format!("/gng/{}", gng_build_shared::BUILD_SCRIPT));

    let script = format!(
        r#"
package.path = "/gng/lua/?.lua"
require("startup").init("{}")"#,
        build_file.to_string_lossy().as_ref()
    );

    evaluate::<()>(lua, &script)?;

    Ok(())
}

// ----------------------------------------------------------------------
// - LuaScriptSupport:
// ----------------------------------------------------------------------

/// Script support for Lua
pub(crate) struct LuaScriptSupport {
    /// The Lua engine
    lua: rlua::Lua,
}

impl crate::script_support::ScriptSupport for LuaScriptSupport {
    fn parse_build_script(&mut self) -> eyre::Result<gng_build_shared::SourcePacket> {
        evaluate::<gng_build_shared::SourcePacket>(&mut self.lua, "PKG")
    }
    fn prepare(&mut self) -> eyre::Result<()> {
        evaluate::<()>(&mut self.lua, "prepare()")
    }

    fn build(&mut self) -> eyre::Result<()> {
        evaluate::<()>(&mut self.lua, "build()")
    }

    fn check(&mut self) -> eyre::Result<()> {
        evaluate::<()>(&mut self.lua, "check()")
    }

    fn install(&mut self) -> eyre::Result<()> {
        evaluate::<()>(&mut self.lua, "install()")
    }

    fn polish(&mut self) -> eyre::Result<()> {
        evaluate::<()>(&mut self.lua, "polish()")
    }
}

impl LuaScriptSupport {
    /// Create a new `LuaScriptSupport`
    pub(crate) fn new() -> eyre::Result<Self> {
        let mut lua = rlua::Lua::new();

        set_max_operations(&mut lua, 4000);
        set_max_memory(&mut lua, 4 * 1024 * 1024);

        push_string_constant(
            &mut lua,
            "WORK_DIR",
            std::fs::canonicalize(crate::take_env(
                ce::GNG_WORK_DIR,
                cc::GNG_WORK_DIR.to_str().unwrap(),
            ))
            .wrap_err("Failed to turn WORK_DIR into canonical form")?
            .to_string_lossy()
            .as_ref(),
        )?;
        push_string_constant(
            &mut lua,
            "INST_DIR",
            std::fs::canonicalize(crate::take_env(
                ce::GNG_INST_DIR,
                cc::GNG_INST_DIR.to_str().unwrap(),
            ))
            .wrap_err("Failed to turn INST_DIR into canonical form")?
            .to_string_lossy()
            .as_ref(),
        )?;

        setup_lua(&mut lua)?;

        eval_pkgsrc_directory(&mut lua)?;

        Ok(Self { lua })
    }
}
