// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use super::query_handler::SourcePacketHandle;
use crate::handler::Handler;

use gng_core::Name;

use eyre::Result;

// ----------------------------------------------------------------------
// - InstallHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct InstallHandler {
    source_packet: SourcePacketHandle,
    root_directory: std::path::PathBuf,
    _installed_packages: std::collections::HashSet<Name>,
}

impl InstallHandler {
    /// Create a new `InstallHandler`
    pub fn new(source_packet: SourcePacketHandle, root_directory: &std::path::Path) -> Self {
        Self {
            source_packet,
            root_directory: root_directory.to_path_buf(),
            _installed_packages: std::collections::HashSet::new(),
        }
    }
}

impl Handler for InstallHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        if *mode != crate::Mode::Install && *mode != crate::Mode::Check {
            return Ok(());
        }

        let source_packet = self.source_packet.borrow();
        let source_packet = source_packet
            .as_ref()
            .expect("SourcePacket should be defined here.");

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
            &self.root_directory.to_string_lossy(),
        );

        while !to_install.is_empty() {
            todo!()
            // gng_packet_io::PacketReader::new(i)
        }

        Ok(())
    }
}
