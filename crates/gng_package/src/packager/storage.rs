// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Packager` that stores data into a `Packet`

use crate::path::Path;
use crate::{packager::Packager, FacetDefinition, PacketDefinition};

use gng_packet_writer::{create_packet_writer, packet_writer::BoxedPacketWriter};

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

const fn find_policy(
    packet: &PacketDefinition,
    facet: &FacetDefinition,
) -> gng_packet_writer::PacketPolicy {
    if packet.is_empty {
        gng_packet_writer::PacketPolicy::MustStayEmpty
    } else if facet.name.is_some() {
        gng_packet_writer::PacketPolicy::MayHaveContents
    } else {
        gng_packet_writer::PacketPolicy::MustHaveContents
    }
}

// ----------------------------------------------------------------------
// - StoragePackager:
// ----------------------------------------------------------------------

/// A `Packager` that can select between a set of `children` `Packager`
pub struct StoragePackager {
    debug: String,
    writer: BoxedPacketWriter,
}

impl StoragePackager {
    /// Constructor
    ///
    /// # Errors
    ///
    /// Returns an error if one happens.
    pub fn new(packet: &PacketDefinition, facet: &FacetDefinition) -> eyre::Result<Self> {
        Ok(Self {
            debug: packet.name.combine(&facet.name),
            writer: create_packet_writer(
                std::path::Path::new("."),
                &packet.name,
                &facet.name,
                &packet.version,
                packet.metadata.clone(),
                find_policy(packet, facet),
            )?,
        })
    }
}

impl Packager for StoragePackager {
    #[tracing::instrument(level = "trace", skip(self))]
    fn package(&mut self, path: &Path) -> eyre::Result<bool> {
        tracing::trace!("Packaging in {}.", &self.debug_name());
        let size = path.size();
        let mode = path.mode();
        let user_id = u64::from(path.user_id());
        let group_id = u64::from(path.group_id());

        match path.leaf_type() {
            "d" => self
                .writer
                .add_directory(path.as_path(), mode, user_id, group_id)
                .map(|_| true),
            "l" => self
                .writer
                .add_link(
                    path.as_path(),
                    &path.link_target().expect("Must be set for links"),
                )
                .map(|_| true),
            "f" => match path.file_contents().expect("Files have contents!") {
                crate::path::FileContents::Buffer(d) => self
                    .writer
                    .add_buffer(path.as_path(), d, mode, user_id, group_id)
                    .map(|_| true),
                crate::path::FileContents::OnDisk(p) => self
                    .writer
                    .add_file(path.as_path(), p, size, mode, user_id, group_id)
                    .map(|_| true),
            },
            _ => unreachable!("Path type is not supported."),
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn finish(&mut self) -> eyre::Result<Vec<std::path::PathBuf>> {
        tracing::trace!("Finishing in {}.", &self.debug_name());
        Ok((self.writer.finish()?).map_or_else(Vec::new, |path| vec![path]))
    }

    fn debug_name(&self) -> String {
        format!("[ Storage {} ]", &self.debug)
    }
}
