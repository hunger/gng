// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use crate::handler::Handler;

use gng_build_shared::SourcePacket;

use eyre::Result;

// ----------------------------------------------------------------------
// - QueryHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct QueryHandler {
    source_packet: std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>>,
}

impl Default for QueryHandler {
    fn default() -> Self {
        Self {
            source_packet: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }
}

impl QueryHandler {
    /// Return the `SourcePacketInfo`
    pub fn source_packet(&self) -> std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>> {
        self.source_packet.clone()
    }
}

impl Handler for QueryHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if *mode == crate::Mode::Query && message_type == &gng_build_shared::MessageType::Data {
            tracing::debug!("Setting source packet info in QueryHandler.");
            self.source_packet
                .replace(Some(serde_json::from_str(message)?));
        }

        Ok(false)
    }
}
