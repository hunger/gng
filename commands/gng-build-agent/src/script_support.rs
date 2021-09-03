// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Add support for scripting to `gng-build-agent`

// ----------------------------------------------------------------------
// - ScriptSupport:
// ----------------------------------------------------------------------

/// Support for a scripting language
pub trait ScriptSupport {
    /// Get the `SourcePacket` description from the `ScriptSupport`
    ///
    /// # Errors
    /// Return some error if something fails.
    fn parse_build_script(&mut self) -> eyre::Result<gng_build_shared::SourcePacket>;

    /// Run the `prepare` function
    ///
    /// # Errors
    /// Return some error if something fails.
    fn prepare(&mut self) -> eyre::Result<()>;

    /// Run the `build` function
    ///
    /// # Errors
    /// Return some error if something fails.
    fn build(&mut self) -> eyre::Result<()>;

    /// Run the `check` function
    ///
    /// # Errors
    /// Return some error if something fails.
    fn check(&mut self) -> eyre::Result<()>;

    /// Run the `install` function
    ///
    /// # Errors
    /// Return some error if something fails.
    fn install(&mut self) -> eyre::Result<()>;

    /// Run the `polish` function
    ///
    /// # Errors
    /// Return some error if something fails.
    fn polish(&mut self) -> eyre::Result<()>;
}
