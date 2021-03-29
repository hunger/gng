// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::packet::PacketWriterFactory;

use super::facet::Facet;

// - Helper:
// ----------------------------------------------------------------------

fn same_packet_name(packet: &Packet, packets: &[Packet]) -> bool {
    packets.iter().any(|p| p.path == packet.path)
}

pub fn validate_packets(packet: &Packet, packets: &[Packet]) -> gng_shared::Result<()> {
    // TODO: More sanity checking!
    if same_packet_name(packet, packets) {
        return Err(gng_shared::Error::Runtime {
            message: format!(
                "Duplicate packet entry {} found.",
                packet.path.to_string_lossy()
            ),
        });
    }
    Ok(())
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

pub struct Packet {
    pub path: std::path::PathBuf,
    pub data: gng_shared::Packet,
    pub patterns: Vec<glob::Pattern>,
    pub writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
    pub facets: Vec<Facet>,
}

impl Packet {
    pub fn new(
        path: &std::path::Path,
        data: &gng_shared::Packet,
        patterns: Vec<glob::Pattern>,
    ) -> Self {
        Self {
            path: path.to_owned(),
            data: data.clone(),
            patterns,
            writer: None,
            facets: Facet::facets_from(path, data),
        }
    }

    pub fn contains(&self, packaged_path: &gng_shared::packet::Path, _mime_type: &str) -> bool {
        let packaged_path = packaged_path.path();
        self.patterns.iter().any(|p| p.matches_path(&packaged_path))
    }

    pub fn store_path(
        &mut self,
        factory: &PacketWriterFactory,
        packet_path: &mut gng_shared::packet::Path,
        mime_type: &str,
    ) -> gng_shared::Result<()> {
        let facet = self
            .facets
            .iter_mut()
            .find(|f| f.contains(packet_path, mime_type))
            .ok_or(gng_shared::Error::Runtime {
                message: "No facet found!".to_string(),
            })?;
        facet.store_path(factory, packet_path, mime_type)
    }

    pub fn finish(&mut self) -> gng_shared::Result<Vec<std::path::PathBuf>> {
        let mut result = Vec::with_capacity(self.facets.len());

        for f in &mut self.facets {
            result.append(&mut f.finish()?);
        }
        Ok(result)
    }
}
