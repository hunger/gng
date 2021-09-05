// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use crate::handler::Handler;

use gng_build_shared::SourcePacket;

use std::convert::TryFrom;

use eyre::Result;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn verify_source_packet(source_packet: &SourcePacket) -> Result<()> {
    // FIXME: Actually validate source packet

    if source_packet.name == gng_core::Name::try_from("foobar").unwrap() {
        return Err(eyre::eyre!("Source Packet name is foobar!"));
    }
    Ok(())
}

// ----------------------------------------------------------------------
// - VerifySourcePacketHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct VerifySourcePacketHandler {
    source_packet: std::rc::Rc<Option<SourcePacket>>,
}

impl VerifySourcePacketHandler {
    /// Create a new `VerifySourcePacketHandler`
    pub fn new(source_packet: std::rc::Rc<Option<SourcePacket>>) -> Self {
        Self { source_packet }
    }
}

impl Handler for VerifySourcePacketHandler {
    #[tracing::instrument(level = "trace", skip(self))]
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
        if *mode == crate::Mode::Query && message_type == &gng_build_shared::MessageType::Data {
            let source_packet = &(*self.source_packet).as_ref().expect(
                "VerifySourcePacketHandler let this go through, so there is a source_packet set",
            );
            verify_source_packet(source_packet).map(|_| true)
        } else {
            Ok(true)
        }
    }
}
