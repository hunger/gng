// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The script engine driving the `gng-build-agent`

use gng_shared::package::{Hash, Name, Version};

use rhai::{ImmutableString, RegisterResultFn};

use std::convert::TryFrom;
use std::path::Path;

// - Helpers:
// ----------------------------------------------------------------------

fn register_simple_type<T>(engine: &mut rhai::Engine)
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    engine.register_type::<T>();
}

fn register_custom_types(engine: &mut rhai::Engine) {
    register_simple_type::<Hash>(engine);
    engine.register_result_fn("h", hash_constructor);
    register_simple_type::<Name>(engine);
    engine.register_result_fn("n", name_constructor);
    register_simple_type::<Version>(engine);
    engine.register_result_fn("v", version_constructor);
}

// - Custom Functions:
// ----------------------------------------------------------------------

fn name_constructor(
    name: ImmutableString,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    match Name::try_from(name.to_string()) {
        Err(e) => Err(e.to_string().into()),
        Ok(v) => Ok(rhai::Dynamic::from(v)),
    }
}

fn version_constructor(
    version: ImmutableString,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    match Version::try_from(version.to_string()) {
        Err(e) => Err(e.to_string().into()),
        Ok(v) => Ok(rhai::Dynamic::from(v)),
    }
}

fn hash_constructor(
    version: ImmutableString,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    match Hash::try_from(version.to_string()) {
        Err(e) => Err(e.to_string().into()),
        Ok(v) => Ok(rhai::Dynamic::from(v)),
    }
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
    pub fn push_constant(&mut self, key: &str, value: rhai::Dynamic) -> &mut Self {
        self.scope.push_constant(String::from(key), value);
        self
    }

    /// Evaluate a script fil
    pub fn eval_pkgsrc_directory(&mut self, pkgsrc_dir: &Path) -> crate::Result<Engine<'a>> {
        let mut engine = std::mem::replace(&mut self.engine, rhai::Engine::new());
        let mut scope = std::mem::replace(&mut self.scope, rhai::Scope::<'a>::new());

        // Push default values
        scope.push("bug_url", String::new());

        let build_file = Path::new(pkgsrc_dir).join("build.rhai");
        let build_file_str = build_file.to_string_lossy().into_owned();

        register_custom_types(&mut engine);

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
        T: Clone + Send + Sync + 'static,
    {
        self.engine
            .eval_with_scope::<T>(&mut self.scope, expression)
            .map_err(|e| {
                crate::Error::Script(
                    format!("Failed to evaluate expression {}", expression),
                    e.to_string(),
                )
            })
    }

    /// Evaluate an expression holing an Array
    pub fn evaluate_array<T>(&mut self, expression: &str) -> crate::Result<Vec<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        let array = self.evaluate::<rhai::Array>(expression)?;

        let mut result = Vec::<T>::with_capacity(array.len());
        for d in array {
            let t = d.try_cast::<T>();
            if t.is_none() {
                return Err(crate::Error::Conversion(format!(
                    "Failed to convert Array value when evaluating {}",
                    expression
                )));
            }
            result.push(t.unwrap());
        }

        Ok(result)
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
