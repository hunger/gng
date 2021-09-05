// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use crate::handler::Handler;

use gng_build_shared::SourcePacket;

use eyre::Result;

// ----------------------------------------------------------------------
// - SourcesHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct SourcesHandler {
    source_packet: std::rc::Rc<Option<SourcePacket>>,
}

impl SourcesHandler {
    /// Return the `SourcePacketInfo`
    pub fn new(source_packet: std::rc::Rc<Option<SourcePacket>>) -> Self {
        Self { source_packet }
    }
}

impl Handler for SourcesHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        if *mode != crate::Mode::Build {
            return Ok(());
        }

        let source_packet = &(*self.source_packet)
            .as_ref()
            .expect("QueryHandler let this go through, so there is a source_packet set");

        let to_install = source_packet.sources.clone();

        if to_install.is_empty() {
            return Ok(());
        }

        tracing::info!("Fetching sources.");

        // FIXME: Actually fetch sources;-)

        Ok(())
    }
}
