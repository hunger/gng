// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The script engine driving the `gng-build-agent`

use gng_shared::{Hash, Version};

use rhai::{Dynamic, EvalAltResult, ImmutableString, RegisterResultFn};

use std::convert::TryFrom;
use std::path::Path;
use std::string::ToString;

// - Helpers:
// ----------------------------------------------------------------------

fn register_custom_functionality(engine: &mut rhai::Engine) {
    engine
        .register_result_fn("version_epoch", version_epoch)
        .register_result_fn("version_upstream", version_upstream)
        .register_result_fn("version_release", version_release)
        .register_result_fn("hash_algorithm", hash_algorithm)
        .register_result_fn("hash_value", hash_value);
}

// - Custom Functions:
// ----------------------------------------------------------------------

fn version_epoch(input: ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
    let version = Version::try_from(input.to_string()).map_err(|e| e.to_string())?;
    Ok(Dynamic::from(version.epoch()))
}

fn version_upstream(input: ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
    let version = Version::try_from(input.to_string()).map_err(|e| e.to_string())?;
    Ok(version.upstream().into())
}

fn version_release(input: ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
    let version = Version::try_from(input.to_string()).map_err(|e| e.to_string())?;
    Ok(version.release().into())
}

fn hash_algorithm(input: ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
    let hash = Hash::try_from(input.to_string()).map_err(|e| e.to_string())?;
    Ok(hash.algorithm().into())
}

fn hash_value(input: ImmutableString) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
    let hash = Hash::try_from(input.to_string()).map_err(|e| e.to_string())?;
    Ok(hash.value().into())
}

// ----------------------------------------------------------------------
// - Engine:
// ----------------------------------------------------------------------

/// A builder for `Engine`
pub struct EngineBuilder<'a> {
    engine: rhai::Engine,
    scope: rhai::Scope<'a>,
}

impl<'a> Default for EngineBuilder<'a> {
    fn default() -> Self {
        Self {
            engine: rhai::Engine::new(),
            scope: rhai::Scope::<'a>::new(),
        }
    }
}

impl<'a> EngineBuilder<'a> {
    /// Set max operations on engine
    pub fn set_max_operations(&mut self, count: u64) -> &mut Self {
        self.engine.set_max_operations(count);
        self
    }

    /// Push a constant into the `Engine`
    pub fn push_constant(&mut self, key: &str, value: Dynamic) -> &mut Self {
        self.scope.push_constant(String::from(key), value);
        self
    }

    /// Evaluate a script fil
    pub fn eval_pkgsrc_directory(&mut self, pkgsrc_dir: &Path) -> crate::Result<Engine<'a>> {
        let mut engine = std::mem::replace(&mut self.engine, rhai::Engine::new());
        let mut scope = std::mem::replace(&mut self.scope, rhai::Scope::<'a>::new());

        let build_file = Path::new(pkgsrc_dir).join("build.rhai");
        let build_file_str = build_file.to_string_lossy().into_owned();

        register_custom_functionality(&mut engine);

        let ast = engine
            .compile_file_with_scope(&mut scope, build_file)
            .map_err(|e| {
                crate::Error::Script(
                    format!("Compilation of build script {} failed", build_file_str),
                    e.to_string(),
                )
            })?;
        engine.eval_ast_with_scope(&mut scope, &ast).map_err(|e| {
            crate::Error::Script(
                format!("Evaluation of build script {} failed", build_file_str),
                e.to_string(),
            )
        })?;

        Ok(Engine { engine, scope, ast })
    }
}

/// The script Engine driving the `gng-build-agent`
pub struct Engine<'a> {
    engine: rhai::Engine,
    scope: rhai::Scope<'a>,
    ast: rhai::AST,
}

impl<'a> Engine<'a> {
    /// Evaluate an expression
    pub fn evaluate<T>(&mut self, expression: &str) -> crate::Result<T>
    where
        T: Clone + Send + Sync + serde::de::DeserializeOwned + 'static,
    {
        let result = self
            .engine
            .eval_with_scope::<Dynamic>(&mut self.scope, expression)
            .map_err(|e| {
                crate::Error::Script(
                    format!("Failed to evaluate expression {}", expression),
                    e.to_string(),
                )
            })?;
        rhai::serde::from_dynamic::<T>(&result).map_err(|e| crate::Error::Conversion(e.to_string()))
    }

    /// Call a function (without arguments!)
    pub fn call<T>(&mut self, name: &str) -> crate::Result<T>
    where
        T: Clone + Sync + Send + 'static,
    {
        self.engine
            .call_fn(&mut self.scope, &mut self.ast, name, ())
            .map_err(|e| {
                crate::Error::Script(format!("Failed to call function {}", name), e.to_string())
            })
    }
}
