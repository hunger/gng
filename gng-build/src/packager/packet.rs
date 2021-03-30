// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use super::facet::{Facet, FacetDefinition};

// - Helper:
// ----------------------------------------------------------------------

fn same_packet_name(packet: &PacketBuilder, packets: &[PacketBuilder]) -> bool {
    packets
        .iter()
        .any(|p| -> bool { p.data.name == packet.data.name })
}

pub fn validate_packets(
    packet: &PacketBuilder,
    packets: &[PacketBuilder],
) -> gng_shared::Result<()> {
    // TODO: More sanity checking!
    if same_packet_name(packet, packets) {
        return Err(gng_shared::Error::Runtime {
            message: "Duplicate packet entry found.".to_string(),
        });
    }
    Ok(())
}

// ----------------------------------------------------------------------
// - PacketBuilder:
// ----------------------------------------------------------------------

pub struct PacketBuilder {
    pub data: gng_shared::Packet,
    pub patterns: Vec<glob::Pattern>,
}

impl PacketBuilder {
    pub fn new(data: &gng_shared::Packet, patterns: Vec<glob::Pattern>) -> Self {
        Self {
            data: data.clone(),
            patterns,
        }
    }

    pub fn build(self, facet_definitions: &[FacetDefinition]) -> gng_shared::Result<Packet> {
        Packet::new(self.data, self.patterns, facet_definitions)
    }
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

pub struct Packet {
    pub data: gng_shared::Packet,
    pub patterns: Vec<glob::Pattern>,
    pub writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
    pub facets: Vec<Facet>,
}

impl Packet {
    fn new(
        data: gng_shared::Packet,
        patterns: Vec<glob::Pattern>,
        facet_definitions: &[FacetDefinition],
    ) -> gng_shared::Result<Self> {
        let facets = Facet::facets_from(facet_definitions, &data)?;

        Ok(Self {
            data,
            patterns,
            writer: None,
            facets,
        })
    }

    pub fn contains(&self, path: &std::path::Path, _mime_type: &str) -> bool {
        self.patterns.iter().any(|p| p.matches_path(path))
    }

    pub fn store_path(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
        package_path: &mut gng_shared::packet::Path,
        mime_type: &str,
    ) -> gng_shared::Result<()> {
        let path = package_path.path();
        let facet = self
            .facets
            .iter_mut()
            .find(|f| f.contains(&path, mime_type))
            .ok_or(gng_shared::Error::Runtime {
                message: "No facet found!".to_string(),
            })?;
        facet.store_path(factory, package_path, mime_type)
    }

    pub fn finish(&mut self) -> gng_shared::Result<Vec<std::path::PathBuf>> {
        let mut result = Vec::with_capacity(self.facets.len());

        for f in &mut self.facets {
            result.append(&mut f.finish()?);
        }
        Ok(result)
    }
}
