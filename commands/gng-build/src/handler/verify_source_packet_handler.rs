// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use crate::handler::Handler;

use gng_build_shared::{PacketDefinition, SourceDefinition, SourcePacket};

use eyre::{eyre, Result, WrapErr};

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn verify_packet(packet: &PacketDefinition) -> Result<()> {
    if packet.description.is_empty() {
        Err(eyre!(
            "The packet \"{}\" needs a `description`.",
            &packet.name,
        ))
    } else {
        let files: Vec<_> = packet.files.iter().map(String::as_str).collect();
        gng_package::strings_to_globs(&files[..]).wrap_err(eyre!(
            "The `files` of packet \"{}\" contains an invalid glob pattern.",
            &packet.name,
        ))?;
        Ok(())
    }
}

fn verify_packets(packets: &[gng_build_shared::PacketDefinition]) -> Result<()> {
    if packets.is_empty() {
        Err(eyre!("At least one packet needs to be defined."))
    } else {
        for p in packets {
            verify_packet(p)?;
        }
        Ok(())
    }
}

fn valid_file_path(path: &str) -> bool {
    !path.starts_with('/') && !path.starts_with("../") && !path.contains("/../")
}

fn verify_source(source: &SourceDefinition) -> Result<()> {
    url::Url::parse(&source.source)
        .wrap_err(eyre!("`source` \"{}\" in invalid.", source.source))?;
    for s in &source.mirrors {
        url::Url::parse(&source.source).wrap_err(eyre!(
            "\"{}\" has an invalid `mirror` \"{}\".",
            source.source,
            s,
        ))?;
    }
    if valid_file_path(&source.destination) {
        Ok(())
    } else {
        Err(eyre!(
            "\"{}\" has an invalid `destination` \"{}\".",
            &source.source,
            &source.destination
        ))
    }
}

fn verify_sources(sources: &[gng_build_shared::SourceDefinition]) -> Result<()> {
    for s in sources {
        verify_source(s)?;
    }
    Ok(())
}

fn verify_source_packet(source_packet: &SourcePacket) -> Result<()> {
    if source_packet.license.is_empty() {
        Err(eyre!("The Source definition must include a `license`."))
    } else if source_packet.url.is_empty() {
        Err(eyre!("The Source definition must include an `url`."))
    } else if source_packet.bug_url.is_empty() {
        Err(eyre!("The Source definition must include a `bug_url`."))
    } else if source_packet.description.is_empty() {
        Err(eyre!("The Source definition must include a `description`."))
    } else {
        url::Url::parse(&source_packet.url).wrap_err(eyre!(
            "The source definition included an invalid `url` \"{}\".",
            &source_packet.url
        ))?;
        url::Url::parse(&source_packet.bug_url).wrap_err(eyre!(
            "The source definition included an invalid `bug_url` \"{}\".",
            &source_packet.url
        ))?;
        spdx::Expression::parse(&source_packet.license)
            .map_err(|e| eyre!(e.to_string()))
            .wrap_err("`license` is invalid.")?;

        verify_packets(&source_packet.packets).wrap_err(eyre!(
            "The source definition contains an invalid `packets` definition."
        ))?;
        verify_sources(&source_packet.sources).wrap_err(eyre!(
            "The source definition contains an invalid `sources` definition."
        ))?;
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - VerifySourcePacketHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct VerifySourcePacketHandler {
    source_packet: std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>>,
}

impl VerifySourcePacketHandler {
    /// Create a new `VerifySourcePacketHandler`
    pub fn new(source_packet: std::rc::Rc<std::cell::RefCell<Option<SourcePacket>>>) -> Self {
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
            tracing::debug!("Verifying source packet info.");
            let source_packet = self.source_packet.borrow();
            let source_packet = source_packet
                .as_ref()
                .expect("SourcePacket should be defined here.");

            verify_source_packet(source_packet)?;
        }

        Ok(false)
    }
}
