// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use super::{
    facet::{Facet, MainFacet},
    PacketFiles,
};

// - Helper:
// ----------------------------------------------------------------------

fn same_packet_name(packet: &PacketBuilder, packets: &[PacketBuilder]) -> bool {
    packets
        .iter()
        .any(|p| -> bool { p.data.name == packet.data.name })
}

pub fn validate_packets(packet: &PacketBuilder, packets: &[PacketBuilder]) -> eyre::Result<()> {
    // TODO: More sanity checking!
    if same_packet_name(packet, packets) {
        return Err(eyre::eyre!("Duplicate packet entry found."));
    }
    Ok(())
}

// ----------------------------------------------------------------------
// - PacketBuilder:
// ----------------------------------------------------------------------

#[derive(Debug)]
pub struct PacketBuilder {
    pub data: gng_shared::PacketFileData,
    pub patterns: Vec<glob::Pattern>,
    pub contents_policy: crate::ContentsPolicy,
}

impl PacketBuilder {
    pub fn new(
        data: &gng_shared::PacketFileData,
        patterns: Vec<glob::Pattern>,
        contents_policy: crate::ContentsPolicy,
    ) -> Self {
        Self {
            data: data.clone(),
            patterns,
            contents_policy,
        }
    }

    pub fn build(self, facet_definitions: &[super::NamedFacet]) -> eyre::Result<Packet> {
        Packet::new(
            self.data,
            self.patterns,
            facet_definitions,
            self.contents_policy,
        )
    }
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

pub struct Packet {
    pub data: gng_shared::PacketFileData,
    pub patterns: Vec<glob::Pattern>,
    pub writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
    pub facets: Vec<Facet>,
    pub main_facet: MainFacet,
}

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let patterns: Vec<String> = self.patterns.iter().map(glob::Pattern::to_string).collect();
        write!(
            f,
            "Packet {{ name: {:?}, patterns: {:?} }}",
            &self.full_debug_name(),
            &patterns
        )
    }
}

impl Packet {
    fn full_debug_name(&self) -> String {
        format!(
            "{}-{}",
            &self.data.name.to_string(),
            &self.data.version.to_string(),
        )
    }

    #[tracing::instrument(level = "trace")]
    fn new(
        data: gng_shared::PacketFileData,
        patterns: Vec<glob::Pattern>,
        facet_definitions: &[super::NamedFacet],
        contents_policy: crate::ContentsPolicy,
    ) -> eyre::Result<Self> {
        let (facets, main_facet) = Facet::facets_from(facet_definitions, &data, contents_policy)?;

        Ok(Self {
            data,
            patterns,
            writer: None,
            facets,
            main_facet,
        })
    }

    #[tracing::instrument(level = "trace")]
    pub fn contains(&self, path: &std::path::Path, mime_type: &str) -> bool {
        let result = self.patterns.iter().any(|p| p.matches_path(path));
        tracing::debug!("{:?} contains {:?}? {}", self, path, result);
        result
    }

    #[allow(clippy::option_if_let_else)]
    #[tracing::instrument(level = "trace", skip(factory))]
    pub fn store_path(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
        package_path: &mut gng_shared::packet::Path,
        mime_type: &str,
    ) -> eyre::Result<()> {
        let path = package_path.path();
        let facet = self
            .facets
            .iter_mut()
            .find(|f| f.contains(&path, mime_type));

        if let Some(facet) = facet {
            facet.store_path(factory, package_path, mime_type)
        } else {
            self.main_facet.store_path(factory, package_path, mime_type)
        }
    }

    #[tracing::instrument(level = "trace", skip(factory))]
    pub fn finish(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
    ) -> eyre::Result<PacketFiles> {
        let facets: Vec<Option<(gng_shared::Name, std::path::PathBuf, gng_shared::Hash)>> = self
            .facets
            .iter_mut()
            .map(Facet::finish)
            .collect::<eyre::Result<Vec<_>>>()?;

        let mut files = facets
            .iter()
            .filter_map(|f| f.as_ref().map(|(_, p, h)| (p.clone(), h.clone())))
            .collect::<Vec<_>>();
        let facets = facets
            .iter()
            .filter_map(|f| f.as_ref().map(|(n, _, h)| (n.clone(), h.clone())))
            .collect::<Vec<_>>();

        let (packet, path, hash) = self.main_facet.finish(&facets[..], factory)?;
        files.push((path, hash));

        let result = PacketFiles { packet, files };

        Ok(result)
    }
}
