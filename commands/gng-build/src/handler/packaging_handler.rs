// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use crate::handler::Handler;

use gng_build_shared::SourcePacket;

use eyre::Result;

// ----------------------------------------------------------------------
// - PackagingHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct PackagingHandler {
    source_packet: std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>>,
    root_directory: std::path::PathBuf,
    install_directory: std::path::PathBuf,
}

impl PackagingHandler {
    /// Create a new `PackagingHandler`
    pub fn new(
        source_packet: std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>>,
        root_directory: &std::path::Path,
        install_directory: &std::path::Path,
    ) -> Self {
        Self {
            source_packet,
            root_directory: root_directory.to_path_buf(),
            install_directory: install_directory.to_path_buf(),
        }
    }
}

impl Handler for PackagingHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn clean_up(&mut self, mode: &crate::Mode) -> Result<()> {
        if *mode != crate::Mode::Package {
            return Ok(());
        }

        let source_packet = self.source_packet.borrow();
        let source_packet = source_packet
            .as_ref()
            .expect("SourcePacket should be defined here.");

        tracing::info!(
            "Packaging files in  \"{}\".",
            &self.install_directory.to_string_lossy(),
        );

        // FIXME: Actually install packets

        Ok(())
    }
}
