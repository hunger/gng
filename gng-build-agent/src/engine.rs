// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The script engine driving the `gng-build-agent`

use std::string::ToString;

// - Helpers:
// ----------------------------------------------------------------------

// fn register_custom_functionality(engine: &mut rhai::Engine) {
//     // Create plugin module.
//     let fs_module = std::sync::Arc::new(rhai::plugin::exported_module!(rhai_modules::fs_module));
//     engine.register_global_module(fs_module);

//     engine
//         .register_result_fn("version_epoch", version_epoch)
//         .register_result_fn("version_upstream", version_upstream)
//         .register_result_fn("version_release", version_release)
//         .register_result_fn("hash_algorithm", hash_algorithm)
//         .register_result_fn("hash_value", hash_value);
// }

fn map_error(error: &mlua::Error) -> crate::Error {
    match error {
        mlua::Error::SyntaxError {
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
            )
            .to_string(),
        ),
        mlua::Error::RuntimeError(msg) => {
            crate::Error::Script("RuntimeError".to_string(), msg.to_string())
        }
        mlua::Error::MemoryError(msg) => {
            crate::Error::Script("MemoryError".to_string(), msg.to_string())
        }
        mlua::Error::RecursiveMutCallback => crate::Error::Script(
            "IntegrationError".to_string(),
            "Recursive call into a mut rust function.".to_string(),
        ),
        mlua::Error::CallbackDestructed => crate::Error::Script(
            "IntegrationError".to_string(),
            "A callback has been destructed too early.".to_string(),
        ),
        mlua::Error::SafetyError(msg) => {
            crate::Error::Script("SafetyError".to_string(), msg.to_string())
        }
        mlua::Error::StackError => crate::Error::Script(
            "IntegrationError".to_string(),
            "Stack was corrupted.".to_string(),
        ),
        mlua::Error::BindError => crate::Error::Script(
            "IntegrationError".to_string(),
            "Binding failed.".to_string(),
        ),
        mlua::Error::ToLuaConversionError { from, to, message } => crate::Error::Script(
            "IntegrationError".to_string(),
            "Conversion to lua failed.".to_string(),
        ),
        mlua::Error::FromLuaConversionError { from, to, message } => crate::Error::Script(
            "IntegrationError".to_string(),
            "Conversion from Lua failed.".to_string(),
        ),
        mlua::Error::CoroutineInactive => crate::Error::Script(
            "IntegrationError".to_string(),
            "A co-routine was inactive.".to_string(),
        ),
        mlua::Error::UserDataTypeMismatch => crate::Error::Script(
            "IntegrationError".to_string(),
            "User data type mismatch.".to_string(),
        ),
        mlua::Error::UserDataBorrowError => crate::Error::Script(
            "IntegrationError".to_string(),
            "User data borrowing problem.".to_string(),
        ),
        mlua::Error::UserDataBorrowMutError => crate::Error::Script(
            "IntegrationError".to_string(),
            "User data mutable borrow error.".to_string(),
        ),
        mlua::Error::MismatchedRegistryKey => crate::Error::Script(
            "IntegrationError".to_string(),
            "iRegistry key mismatch.".to_string(),
        ),
        mlua::Error::CallbackError { traceback, cause } => crate::Error::Script(
            "IntegrationError".to_string(),
            "Callback caused an error.".to_string(),
        ),
        mlua::Error::ExternalError(_) => crate::Error::Script(
            "IntegrationError".to_string(),
            "External error.".to_string(),
        ),
        mlua::Error::MemoryLimitNotAvailable => crate::Error::Script(
            "IntegrationError".to_string(),
            "Memory limit not available.".to_string(),
        ),
        mlua::Error::MainThreadNotAvailable => crate::Error::Script(
            "IntegrationError".to_string(),
            "No main thread.".to_string(),
        ),
        mlua::Error::UserDataDestructed => crate::Error::Script(
            "IntegrationError".to_string(),
            "iUser data was destructed.".to_string(),
        ),
    }
}

// - Custom Functions:
// ----------------------------------------------------------------------

// fn version_epoch(input: &ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
//     let version = Version::try_from(input.to_string()).map_err(|e| e.to_string())?;
//     Ok(Dynamic::from(version.epoch()))
// }

// fn version_upstream(input: &ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
//     let version = Version::try_from(input.to_string()).map_err(|e| e.to_string())?;
//     Ok(version.upstream().into())
// }

// fn version_release(input: &ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
//     let version = Version::try_from(input.to_string()).map_err(|e| e.to_string())?;
//     Ok(version.release().into())
// }

// fn hash_algorithm(input: &ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
//     let hash = Hash::try_from(input.to_string()).map_err(|e| e.to_string())?;
//     Ok(hash.algorithm().into())
// }

// fn hash_value(input: &ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
//     let hash = Hash::try_from(input.to_string()).map_err(|e| e.to_string())?;
//     Ok(hash.value().into())
// }

// ----------------------------------------------------------------------
// - EngineBuilder:
// ----------------------------------------------------------------------

/// A builder for `Engine`
pub struct EngineBuilder {
    lua: mlua::Lua,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            lua: mlua::Lua::new(),
        }
    }
}

impl EngineBuilder {
    /// Set max operations on engine
    pub fn set_max_operations(&mut self, count: u32) -> crate::Result<&mut Self> {
        self.lua
            .set_hook(
                mlua::HookTriggers {
                    every_nth_instruction: Some(count),
                    ..mlua::HookTriggers::default()
                },
                |_lua_context, _debug| {
                    Err(mlua::Error::RuntimeError(
                        "Too many operations!".to_string(),
                    ))
                },
            )
            .map_err(|e| map_error(&e))?;

        Ok(self)
    }

    /// Set max operations on engine
    pub fn set_max_memory(&mut self, size: usize) -> crate::Result<&mut Self> {
        self.lua.set_memory_limit(size).map_err(|e| map_error(&e))?;
        Ok(self)
    }

    /// Push a constant into the `Engine`
    /// # Errors
    /// * Script Error with details on the issue reported by Lua.
    pub fn push_string_constant(&mut self, key: &str, value: &str) -> crate::Result<&mut Self> {
        {
            let globals = self.lua.globals();

            globals.set(key, value).map_err(|e| map_error(&e))?;
        }
        Ok(self)
    }

    /// Evaluate a script file
    ///
    /// # Errors
    /// * `Error::Script`: When the build script is invalid
    pub fn eval_pkgsrc_directory(&mut self) -> crate::Result<Engine> {
        // let mut engine = std::mem::replace(&mut self.engine, rhai::Engine::new());
        // let mut scope = std::mem::replace(&mut self.scope, rhai::Scope::<'a>::new());

        // let build_file = PathBuf::from(format!("/gng/{}", gng_build_shared::BUILD_SCRIPT));
        // let build_file_str = build_file.to_string_lossy().into_owned();

        // register_custom_functionality(&mut engine);

        // let preamble = engine
        //     .compile(
        //         r#"
        //     fn prepare() { }
        //     fn build() { }
        //     fn check() { }
        //     fn install() { }
        //     fn polish() { }
        //     "#,
        //     )
        //     .map_err(|e| {
        //         crate::Error::Script(
        //             String::from("Compilation the preamble failed"),
        //             e.to_string(),
        //         )
        //     })?;

        // let ast = engine
        //     .compile_file_with_scope(&scope, build_file)
        //     .map_err(|e| {
        //         crate::Error::Script(
        //             format!("Compilation of build script {} failed", build_file_str),
        //             e.to_string(),
        //         )
        //     })?;
        // let ast = preamble.merge(&ast);

        // engine.eval_ast_with_scope(&mut scope, &ast).map_err(|e| {
        //     crate::Error::Script(
        //         format!("Evaluation of build script {} failed", build_file_str),
        //         e.to_string(),
        //     )
        // })?;

        // Ok(Engine { engine, scope, ast })
        unimplemented!()
    }
}

// ----------------------------------------------------------------------
// - Engine:
// ----------------------------------------------------------------------

/// The script Engine driving the `gng-build-agent`
pub struct Engine {
    lua: mlua::Lua,
}

impl Engine {
    /// Evaluate an expression
    ///
    /// # Errors
    /// * `Error::Script`: When the expression is invalid
    pub fn evaluate<T>(&mut self, expression: &str) -> crate::Result<T>
    where
        T: Clone + Send + Sync + serde::de::DeserializeOwned + 'static,
    {
        // let result = self
        //     .engine
        //     .eval_with_scope::<Dynamic>(&mut self.scope, expression)
        //     .map_err(|e| {
        //         crate::Error::Script(
        //             format!("Failed to evaluate expression {}", expression),
        //             e.to_string(),
        //         )
        //     })?;
        // rhai::serde::from_dynamic::<T>(&result).map_err(|e| crate::Error::Conversion(e.to_string()))
        unimplemented!()
    }

    /// Call a function (without arguments!)
    ///
    /// # Errors
    /// * `Error::Script`: When the function is not defined in rhai script
    pub fn call<T>(&mut self, name: &str) -> crate::Result<T>
    where
        T: Clone + Sync + Send + 'static,
    {
        // self.engine
        //     .call_fn(&mut self.scope, &self.ast, name, ())
        //     .map_err(|e| {
        //         crate::Error::Script(format!("Failed to call function {}", name), e.to_string())
        //     })
        unimplemented!()
    }
}
