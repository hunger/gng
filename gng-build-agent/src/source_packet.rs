// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `SourcePacket` and related code

// - Helpers:
// ----------------------------------------------------------------------

/// Create a `SourcePacket` from an `Engine`
///
/// # Errors
/// Passes along `Error::Script` from the evaluation
pub fn from_engine(
    engine: &mut crate::engine::Engine,
) -> crate::Result<gng_build_shared::SourcePacket> {
    engine.evaluate::<gng_build_shared::SourcePacket>("PKG")
}
