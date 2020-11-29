// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The script engine driving the `gng-build-agent`

use gng_build_shared::{PacketDefinition, Source};
use gng_shared::{Hash, Name, Version};

use rhai::{ImmutableString, RegisterFn, RegisterResultFn};

use std::convert::TryFrom;
use std::path::Path;
use std::string::ToString;

// - Helpers:
// ----------------------------------------------------------------------

fn register_simple_type<T>(engine: &mut rhai::Engine, name: &str)
where
    T: Clone
        + PartialEq
        + Send
        + Sync
        + std::fmt::Display
        + std::fmt::Debug
        + std::cmp::PartialEq
        + 'static,
{
    engine.register_type_with_name::<T>(name);
    engine
        .register_fn("to_string", |t: &mut T| t.to_string())
        .register_fn("print", |t: &mut T| t.to_string())
        .register_fn("debug", |t: &mut T| format!("{:?}", t))
        .register_fn("+", |s: &str, t: T| format!("{}{}", s, t))
        .register_fn("+", |t: &mut T, s: &str| format!("{}{}", t, s))
        .register_fn("+=", |s: &mut ImmutableString, t: T| {
            *s += ImmutableString::from(t.to_string());
        })
        .register_fn("push", |list: &mut rhai::Array, item: T| {
            list.push(rhai::Dynamic::from(item));
        })
        .register_fn("+=", |list: &mut rhai::Array, item: T| {
            list.push(rhai::Dynamic::from(item));
        })
        .register_fn(
            "insert",
            |list: &mut rhai::Array, position: i64, item: T| {
                if position <= 0 {
                    list.insert(0, rhai::Dynamic::from(item));
                } else if (position as usize) >= list.len() - 1 {
                    list.push(rhai::Dynamic::from(item));
                } else {
                    list.insert(position as usize, rhai::Dynamic::from(item));
                }
            },
        )
        .register_fn("pad", |list: &mut rhai::Array, len: i64, item: T| {
            if len as usize > list.len() {
                list.resize(len as usize, rhai::Dynamic::from(item));
            }
        })
        .register_fn("==", |item1: &mut T, item2: T| item1 == &item2);
}

fn register_custom_types(engine: &mut rhai::Engine) {
    register_simple_type::<Hash>(engine, "Hash");
    engine.register_result_fn("h", hash_constructor);
    register_simple_type::<Name>(engine, "Name");
    engine.register_result_fn("n", name_constructor);
    register_simple_type::<PacketDefinition>(engine, "Packet");
    engine.register_result_fn("packet", packet_constructor);
    register_simple_type::<Source>(engine, "Source");
    engine.register_result_fn("source", source_constructor);
    register_simple_type::<Version>(engine, "Version");
    engine
        .register_get("epoch", version_get_epoch)
        .register_get("version", version_get_version)
        .register_get("release", version_get_release)
        .register_result_fn("v", version_constructor);
}

// - Custom Functions:
// ----------------------------------------------------------------------

fn hash_constructor(
    version: ImmutableString,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    match Hash::try_from(version.to_string()) {
        Err(e) => Err(e.to_string().into()),
        Ok(v) => Ok(rhai::Dynamic::from(v)),
    }
}

fn name_constructor(
    name: ImmutableString,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    match Name::try_from(name.to_string()) {
        Err(e) => Err(e.to_string().into()),
        Ok(v) => Ok(rhai::Dynamic::from(v)),
    }
}

fn packet_constructor(
    input: rhai::Map,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    let result = rhai::serde::from_dynamic::<PacketDefinition>(&rhai::Dynamic::from(input))?;
    Ok(rhai::Dynamic::from(result))
}

fn source_constructor(
    input: rhai::Map,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    let result = rhai::serde::from_dynamic::<Source>(&rhai::Dynamic::from(input))?;
    Ok(rhai::Dynamic::from(result))
}

fn version_constructor(
    version: ImmutableString,
) -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    match Version::try_from(version.to_string()) {
        Err(e) => Err(e.to_string().into()),
        Ok(v) => Ok(rhai::Dynamic::from(v)),
    }
}

fn version_get_epoch(version: &mut Version) -> rhai::INT {
    version.epoch() as rhai::INT
}

fn version_get_version(version: &mut Version) -> String {
    version.version()
}

fn version_get_release(version: &mut Version) -> String {
    version.release()
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
        T: Clone + Send + Sync + serde::de::DeserializeOwned + 'static,
    {
        let array = self.evaluate::<rhai::Array>(expression)?;

        let mut result = Vec::<T>::with_capacity(array.len());
        for d in array.iter() {
            let d = d.flatten_clone();
            let d_str = d.to_string();

            result.push(d.try_cast::<T>().ok_or(crate::Error::Conversion(format!(
                "Failed to cast Array value \"{}\" for expression \"{}\"",
                d_str, expression
            )))?);
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
