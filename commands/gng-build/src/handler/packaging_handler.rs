// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Handler` for `query` Mode

use super::query_handler::SourcePacketHandle;
use crate::handler::Handler;

use gng_build_shared::SourcePacket;
use gng_core::Names;

use eyre::Result;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn calculate_merged_facets(
    _packet: &gng_build_shared::PacketDefinition,
    _source_packet: &SourcePacket,
) -> Names {
    // FIXME: Find merged facets!
    Names::default()
}

fn generate_data(
    source: &SourcePacket,
    packet: &gng_build_shared::PacketDefinition,
) -> gng_package::BinaryPacketDefinition {
    let facet = packet.facet.as_ref().map_or_else(
        || gng_package::BinaryFacet::Main,
        |f| {
            gng_package::BinaryFacet::Definition(gng_package::BinaryFacetDefinition {
                mime_types: f.mime_types.clone(),
                files: f.files.clone(),
                extends: f.extends.clone(),
                is_forbidden: f.is_forbidden,
            })
        },
    );

    gng_package::BinaryPacketDefinition {
        name: packet.name.clone(),
        version: source.version.clone(),
        description: packet.description.clone(),
        url: source.url.clone(),
        bug_url: source.bug_url.clone(),
        dependencies: packet.dependencies.clone(),
        facet,
    }
}

fn generate_packet_definitions(source_packet: &SourcePacket) -> Vec<gng_package::PacketDefinition> {
    source_packet
        .packets
        .iter()
        .map(|p| {
            let merged_facets = calculate_merged_facets(p, source_packet);
            let data = generate_data(source_packet, p);

            gng_package::PacketDefinition::new(
                data,
                merged_facets,
                std::rc::Rc::new(gng_package::filter::GlobFilter::new(
                    gng_package::strings_to_globs(&p.files).expect("This was validated to be OK!"),
                )),
                p.files.is_empty(),
            )
        })
        .collect()
}

fn generate_facet_definitions(_source_packet: &SourcePacket) -> Vec<gng_package::FacetDefinition> {
    // FIXME: Handle facets!
    // `source_packet` is probably the wrong thing to pass in: We need the facets that got put there
    // by the installation step!
    vec![gng_package::FacetDefinition::new(
        None, // Catch-all main facet, Must be last!
        std::rc::Rc::new(gng_package::filter::AlwaysTrue::default()),
    )]
}

// ----------------------------------------------------------------------
// - PackagingHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct PackagingHandler {
    source_packet: SourcePacketHandle,
    install_directory: std::path::PathBuf,
}

impl PackagingHandler {
    /// Create a new `PackagingHandler`
    pub fn new(source_packet: SourcePacketHandle, install_directory: &std::path::Path) -> Self {
        Self {
            source_packet,
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

        tracing::info!(
            "Packaging files in  \"{}\".",
            &self.install_directory.to_string_lossy(),
        );

        let source_packet = self.source_packet.borrow();
        let source_packet = source_packet
            .as_ref()
            .expect("SourcePacket should be defined here.");

        for p in &gng_package::package(
            &self.install_directory,
            &generate_packet_definitions(source_packet),
            &generate_facet_definitions(source_packet),
        )? {
            println!("{}", p.to_string_lossy());
        }

        Ok(())
    }
}
