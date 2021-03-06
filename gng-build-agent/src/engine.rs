// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The script engine driving the `gng-build-agent`

use std::path::PathBuf;

use eyre::WrapErr;

// - Helpers:

fn map_error(error: &rlua::Error) -> gng_shared::Error {
    match error {
        rlua::Error::SyntaxError {
            message,
            incomplete_input,
        } => gng_shared::Error::Script {
            message: format!(
                "SyntaxError: {}{}",
                message,
                if *incomplete_input {
                    " (incomplete)"
                } else {
                    ""
                }
            ),
        },
        rlua::Error::RuntimeError(msg) => gng_shared::Error::Script {
            message: format!("RuntimeError: {}", msg),
        },
        rlua::Error::MemoryError(msg) => gng_shared::Error::Script {
            message: format!("MemoryError: {}", msg),
        },
        rlua::Error::RecursiveMutCallback => gng_shared::Error::Script {
            message: "Recursive call into a mut rust function.".to_string(),
        },
        rlua::Error::CallbackDestructed => gng_shared::Error::Script {
            message: "A callback has been destructed too early.".to_string(),
        },
        rlua::Error::StackError => gng_shared::Error::Script {
            message: "Stack was corrupted.".to_string(),
        },
        rlua::Error::BindError => gng_shared::Error::Script {
            message: "Binding failed.".to_string(),
        },
        rlua::Error::ToLuaConversionError { from, to, message } => gng_shared::Error::Script {
            message: format!(
                "Conversion of \"{}\" to lua \"{}\" failed: {:?}",
                from, to, message
            ),
        },
        rlua::Error::FromLuaConversionError { from, to, message } => gng_shared::Error::Script {
            message: format!(
                "Conversion of Lua \"{}\" to \"{}\" failed: {:?}",
                from, to, message
            ),
        },
        rlua::Error::CoroutineInactive => gng_shared::Error::Script {
            message: "A co-routine was inactive.".to_string(),
        },
        rlua::Error::UserDataTypeMismatch => gng_shared::Error::Script {
            message: "User data type mismatch.".to_string(),
        },
        rlua::Error::UserDataBorrowError => gng_shared::Error::Script {
            message: "User data borrowing problem.".to_string(),
        },
        rlua::Error::UserDataBorrowMutError => gng_shared::Error::Script {
            message: "User data mutable borrow error.".to_string(),
        },
        rlua::Error::MismatchedRegistryKey => gng_shared::Error::Script {
            message: "iRegistry key mismatch.".to_string(),
        },
        rlua::Error::CallbackError {
            traceback: _,
            cause: _,
        } => gng_shared::Error::Script {
            message: "Callback caused an error.".to_string(),
        },
        rlua::Error::ExternalError(_) => gng_shared::Error::Script {
            message: "External error.".to_string(),
        },
        rlua::Error::GarbageCollectorError(_) => gng_shared::Error::Script {
            message: "Garbage collection error.".to_string(),
        },
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
    ///
    /// # Errors
    /// Return error from backend language
    pub fn set_max_operations(&mut self, count: u32) -> eyre::Result<&mut Self> {
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
    ///
    /// # Errors
    /// Return the error from the backend language
    pub fn set_max_memory(&mut self, size: usize) -> eyre::Result<&mut Self> {
        self.lua.set_memory_limit(Some(size));
        Ok(self)
    }

    /// Push a constant into the `Engine`
    ///
    /// # Errors
    /// * Script Error with details on the issue reported by Lua.
    pub fn push_string_constant(&mut self, key: &str, value: &str) -> eyre::Result<&mut Self> {
        {
            self.lua
                .context(|lua_ctx| lua_ctx.globals().set(key, value).map_err(|e| map_error(&e)))?;
        }
        Ok(self)
    }

    /// Evaluate a script file
    ///
    /// # Errors
    /// A generic error when the build script is not valid
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn eval_pkgsrc_directory(&mut self) -> eyre::Result<Engine> {
        let build_file = PathBuf::from(format!("/gng/{}", gng_build_shared::BUILD_SCRIPT));

        let mut engine = Engine {
            lua: std::mem::replace(&mut self.lua, rlua::Lua::new()),
        };

        engine.load_functions()?;

        let script = format!(
            r#"
package.path = "/gng/lua/?.lua"
require("startup").init("{}")"#,
            build_file.to_string_lossy().as_ref()
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
    #[tracing::instrument(level = "trace", skip(self))]
    fn load_functions(&mut self) -> eyre::Result<()> {
        self.lua
            .context(|lua_context| -> std::result::Result<(), rlua::Error> {
                let fn_table = lua_context.create_table()?;

                let chdir_function = lua_context.create_function(|lua_context, path: String| {
                    match std::env::set_current_dir(&path) {
                        Err(e) => Ok((
                            rlua::Value::Nil,
                            lua_context.pack(format!(
                                "Failed to change directory to \"{}\": {}",
                                path, e
                            ))?,
                        )),
                        Ok(()) => Ok((lua_context.pack(true)?, rlua::Value::Nil)),
                    }
                })?;
                fn_table.set("chdir", chdir_function)?;

                let currentdir_function = lua_context.create_function(|lua_context, ()| {
                    match std::env::current_dir() {
                        Err(e) => Ok((
                            rlua::Value::Nil,
                            lua_context.pack(format!("Current directory not found: {}", e))?,
                        )),
                        Ok(d) => Ok((
                            lua_context.pack(d.to_string_lossy().as_ref().to_owned())?,
                            rlua::Value::Nil,
                        )),
                    }
                })?;
                fn_table.set("currentdir", currentdir_function)?;

                let mkdir_function = lua_context.create_function(|lua_context, path: String| {
                    match std::fs::create_dir(&path) {
                        Err(e) => Ok((
                            rlua::Value::Nil,
                            lua_context.pack(format!("Can not create directory: {}", e))?,
                            lua_context.pack(1)?,
                        )),
                        Ok(()) => Ok((lua_context.pack(true)?, rlua::Value::Nil, rlua::Value::Nil)),
                    }
                })?;
                fn_table.set("mkdir", mkdir_function)?;

                let rmdir_function = lua_context.create_function(|lua_context, path: String| {
                    match std::fs::remove_dir(&path) {
                        Err(e) => Ok((
                            rlua::Value::Nil,
                            lua_context.pack(format!("Can not remove directory: {}", e))?,
                            lua_context.pack(1)?,
                        )),
                        Ok(()) => Ok((lua_context.pack(true)?, rlua::Value::Nil, rlua::Value::Nil)),
                    }
                })?;
                fn_table.set("rmdir", rmdir_function)?;

                // Set up Lua side:
                lua_context.globals().set("gng", fn_table)?;

                Ok(())
            })
            .wrap_err("Export of gng functions into Lua runtime failed.")
    }

    /// Evaluate an expression
    ///
    /// # Errors
    /// A generic error if the evaluation fails
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn evaluate<T: serde::de::DeserializeOwned>(
        &mut self,
        expression: &str,
    ) -> eyre::Result<T> {
        self.lua.context(|lua_context| -> eyre::Result<T> {
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

    /// Query whether a function is defined.
    pub fn has_function(&mut self, name: &str) -> bool {
        self.evaluate::<bool>(&format!("type({}) == 'function'", name))
            .unwrap_or(false)
    }
}
