// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The script engine driving the `gng-build-agent`

use std::path::PathBuf;

// - Helpers:

fn map_error(error: &rlua::Error) -> crate::Error {
    match error {
        rlua::Error::SyntaxError {
            message,
            incomplete_input,
        } => crate::Error::Script(
            "SyntaxError".to_string(),
            format!(
                "{}{}",
                message,
                if *incomplete_input {
                    " (incomplete)"
                } else {
                    ""
                }
            ),
        ),
        rlua::Error::RuntimeError(msg) => {
            crate::Error::Script("RuntimeError".to_string(), msg.to_string())
        }
        rlua::Error::MemoryError(msg) => {
            crate::Error::Script("MemoryError".to_string(), msg.to_string())
        }
        rlua::Error::RecursiveMutCallback => crate::Error::Script(
            "IntegrationError".to_string(),
            "Recursive call into a mut rust function.".to_string(),
        ),
        rlua::Error::CallbackDestructed => crate::Error::Script(
            "IntegrationError".to_string(),
            "A callback has been destructed too early.".to_string(),
        ),
        rlua::Error::StackError => crate::Error::Script(
            "IntegrationError".to_string(),
            "Stack was corrupted.".to_string(),
        ),
        rlua::Error::BindError => crate::Error::Script(
            "IntegrationError".to_string(),
            "Binding failed.".to_string(),
        ),
        rlua::Error::ToLuaConversionError {
            from: _,
            to: _,
            message: _,
        } => crate::Error::Script(
            "IntegrationError".to_string(),
            "Conversion to lua failed.".to_string(),
        ),
        rlua::Error::FromLuaConversionError {
            from: _,
            to: _,
            message: _,
        } => crate::Error::Script(
            "IntegrationError".to_string(),
            "Conversion from Lua failed.".to_string(),
        ),
        rlua::Error::CoroutineInactive => crate::Error::Script(
            "IntegrationError".to_string(),
            "A co-routine was inactive.".to_string(),
        ),
        rlua::Error::UserDataTypeMismatch => crate::Error::Script(
            "IntegrationError".to_string(),
            "User data type mismatch.".to_string(),
        ),
        rlua::Error::UserDataBorrowError => crate::Error::Script(
            "IntegrationError".to_string(),
            "User data borrowing problem.".to_string(),
        ),
        rlua::Error::UserDataBorrowMutError => crate::Error::Script(
            "IntegrationError".to_string(),
            "User data mutable borrow error.".to_string(),
        ),
        rlua::Error::MismatchedRegistryKey => crate::Error::Script(
            "IntegrationError".to_string(),
            "iRegistry key mismatch.".to_string(),
        ),
        rlua::Error::CallbackError {
            traceback: _,
            cause: _,
        } => crate::Error::Script(
            "IntegrationError".to_string(),
            "Callback caused an error.".to_string(),
        ),
        rlua::Error::ExternalError(_) => crate::Error::Script(
            "IntegrationError".to_string(),
            "External error.".to_string(),
        ),
        rlua::Error::GarbageCollectorError(_) => crate::Error::Script(
            "GarbadgeCollectorError".to_string(),
            "Garbadge collection error.".to_string(),
        ),
    }
}

// ----------------------------------------------------------------------
// - Traits:
// ----------------------------------------------------------------------

trait EngineValue {
    type Type;
}

// ----------------------------------------------------------------------
// - EngineBuilder:
// ----------------------------------------------------------------------

/// A builder for `Engine`
pub struct EngineBuilder {
    lua: rlua::Lua,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            lua: rlua::Lua::new(),
        }
    }
}

impl EngineBuilder {
    /// Set max operations on engine
    pub fn set_max_operations(&mut self, count: u32) -> crate::Result<&mut Self> {
        self.lua.set_hook(
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

        Ok(self)
    }

    /// Set max operations on engine
    pub fn set_max_memory(&mut self, size: usize) -> crate::Result<&mut Self> {
        self.lua.set_memory_limit(Some(size));
        Ok(self)
    }

    /// Push a constant into the `Engine`
    /// # Errors
    /// * Script Error with details on the issue reported by Lua.
    pub fn push_string_constant(&mut self, key: &str, value: &str) -> crate::Result<&mut Self> {
        {
            self.lua
                .context(|lua_ctx| lua_ctx.globals().set(key, value).map_err(|e| map_error(&e)))?;
        }
        Ok(self)
    }

    /// Evaluate a script file
    ///
    /// # Errors
    /// * `Error::Script`: When the build script is invalid
    pub fn eval_pkgsrc_directory(&mut self) -> crate::Result<Engine> {
        let build_file = PathBuf::from(format!("/gng/{}", gng_build_shared::BUILD_SCRIPT));
        let build_file_str = build_file.to_string_lossy().into_owned();

        let mut engine = Engine {
            lua: std::mem::replace(&mut self.lua, rlua::Lua::new()),
        };

        let script = format!(
            "PKG = {}\n",
            std::fs::read_to_string(build_file).map_err(|e| crate::Error::Script(
                "LoadError".to_string(),
                format!("Failed to read build script: {}", e)
            ))?
        );

        engine.evaluate::<()>(&script)?;

        Ok(engine)
    }
}

// ----------------------------------------------------------------------
// - Engine:
// ----------------------------------------------------------------------

/// The script Engine driving the `gng-build-agent`
pub struct Engine {
    lua: rlua::Lua,
}

impl Engine {
    /// Evaluate an expression
    ///
    /// # Errors
    /// * `Error::Script`: When the expression is invalid
    pub fn evaluate<T: serde::de::DeserializeOwned>(
        &mut self,
        expression: &str,
    ) -> crate::Result<T> {
        tracing::debug!("Evaluating '{}'.", expression);

        self.lua
            .context(|lua_context| {
                let value = lua_context
                    .load(expression)
                    .eval::<rlua::Value>()
                    .expect("evaluable is Infallible");
                rlua_serde::from_value(value)
            })
            .map_err(|e| map_error(&e))
    }
}
