// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Lua support for `gng-build-agent`

use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;

use eyre::WrapErr;

mod engine;

/// Script support for Lua
pub(crate) struct LuaScriptSupport {
    /// The Lua engine
    engine: engine::Engine,
}

impl crate::script_support::ScriptSupport for LuaScriptSupport {
    fn parse_build_script(&mut self) -> eyre::Result<gng_build_shared::SourcePacket> {
        let source_packet = gng_build_shared::SourcePacket::default();
        Ok(source_packet)
    }
    fn prepare(&mut self) -> eyre::Result<()> {
        self.engine.evaluate::<()>("PKG.foo")
    }

    fn build(&mut self) -> eyre::Result<()> {
        Ok(())
    }

    fn check(&mut self) -> eyre::Result<()> {
        Ok(())
    }

    fn install(&mut self) -> eyre::Result<()> {
        Ok(())
    }

    fn polish(&mut self) -> eyre::Result<()> {
        Ok(())
    }
}

impl LuaScriptSupport {
    /// Create a new `LuaScriptSupport`
    pub(crate) fn new() -> eyre::Result<Self> {
        let mut engine_builder = engine::EngineBuilder::default();
        engine_builder
            .set_max_operations(4000)
            .set_max_memory(4 * 1024 * 1024)
            .push_string_constant(
                "WORK_DIR",
                std::fs::canonicalize(crate::take_env(
                    ce::GNG_WORK_DIR,
                    cc::GNG_WORK_DIR.to_str().unwrap(),
                ))
                .wrap_err("Failed to turn WORK_DIR into canonical form")?
                .to_string_lossy()
                .as_ref(),
            )?
            .push_string_constant(
                "INST_DIR",
                std::fs::canonicalize(crate::take_env(
                    ce::GNG_INST_DIR,
                    cc::GNG_INST_DIR.to_str().unwrap(),
                ))
                .wrap_err("Failed to turn INST_DIR into canonical form")?
                .to_string_lossy()
                .as_ref(),
            )?;

        let engine = engine_builder.eval_pkgsrc_directory()?;

        Ok(Self { engine })
    }
}
