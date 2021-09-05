// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use crate::handler::Handler;

use gng_build_shared::SourcePacket;

use eyre::Result;

// ----------------------------------------------------------------------
// - InstallHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct InstallHandler {
    source_packet: std::rc::Rc<Option<SourcePacket>>,
    install_directory: std::path::PathBuf,
}

impl InstallHandler {
    /// Create a new `InstallHandler`
    pub fn new(
        source_packet: std::rc::Rc<Option<SourcePacket>>,
        install_directory: &std::path::Path,
    ) -> Self {
        Self {
            source_packet,
            install_directory: install_directory.to_path_buf(),
        }
    }
}

impl Handler for InstallHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        assert_ne!(*mode, crate::Mode::Query); // QueryHandler makes sure we do not end up here!

        let source_packet = &(*self.source_packet)
            .as_ref()
            .expect("QueryHandler let this go through, so there is a source_packet set");

        let to_install = match *mode {
            crate::Mode::Build => {
                tracing::debug!("Installing build dependencies");
                source_packet.build_dependencies.clone()
            }
            crate::Mode::Check => {
                tracing::debug!("Installing check dependencies");
                source_packet.check_dependencies.clone()
            }
            _ => gng_core::Names::default(),
        };

        if to_install.is_empty() {
            // Nothing to install...
            return Ok(());
        }

        tracing::info!(
            "Installing \"{}\" into {}.",
            &to_install,
            &self.install_directory.to_string_lossy(),
        );

        // FIXME: Actually install packets

        Ok(())
    }
}
