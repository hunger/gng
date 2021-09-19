// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use super::query_handler::SourcePacketHandle;
use crate::handler::Handler;

use gng_build_shared::SourcePacket;

use eyre::Result;

// ----------------------------------------------------------------------
// - SourcesHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct SourcesHandler {
    source_packet: SourcePacketHandle,
    work_directory: std::path::PathBuf,
}

impl SourcesHandler {
    /// Return the `SourcePacketInfo`
    pub fn new(
        source_packet: std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>>,
        work_directory: &std::path::Path,
    ) -> Self {
        Self {
            source_packet,
            work_directory: work_directory.to_path_buf(),
        }
    }
}

impl Handler for SourcesHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        if *mode != crate::Mode::Build {
            return Ok(());
        }

        let source_packet = self.source_packet.borrow();
        let source_packet = source_packet
            .as_ref()
            .expect("SourcePacket should be defined here.");

        let to_install = source_packet.sources.clone();

        if to_install.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "Fetching sources into \"{}\".",
            &self.work_directory.to_string_lossy()
        );

        // FIXME: Actually fetch sources;-)

        Ok(())
    }
}
