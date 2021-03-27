// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::package::PacketWriterFactory;

pub mod deterministic_directory_iterator;
pub mod mimetype_directory_iterator;
pub mod packet;

use mimetype_directory_iterator::MimeTypeDirectoryIterator;

// ----------------------------------------------------------------------
// - Types:
// ----------------------------------------------------------------------

pub struct PacketPath {
    on_disk: std::path::PathBuf,
    in_packet: gng_shared::package::Path,
    mime_type: String,
}

type PackagingIteration = gng_shared::Result<PacketPath>;
type PackagingIterator = dyn Iterator<Item = PackagingIteration>;
type PackagingIteratorFactory =
    dyn Fn(&std::path::Path) -> gng_shared::Result<Box<PackagingIterator>>;

//  ----------------------------------------------------------------------
// - PackagerBuilder:
// ----------------------------------------------------------------------

/// A builder for `Packager`
pub struct PackagerBuilder {
    packet_directory: Option<std::path::PathBuf>,
    packet_factory: Box<PacketWriterFactory>,
    packets: Vec<crate::packager::packet::Packet>,
    iterator_factory: Box<PackagingIteratorFactory>,
}

impl PackagerBuilder {
    /// Set the directory to store packets in.
    ///
    /// # Errors
    /// `gng_shared::Error::Runtime` if this given `path` is not a directory
    pub fn packet_directory(mut self, path: &std::path::Path) -> gng_shared::Result<Self> {
        if !path.is_dir() {
            return Err(gng_shared::Error::Runtime {
                message: format!(
                    "\"{}\" is not a directory, can not store packets there.",
                    path.to_string_lossy()
                ),
            });
        }

        self.packet_directory = Some(path.to_owned());
        Ok(self)
    }

    /// Add a packet
    ///
    /// # Errors
    /// `gng_shared::Error::Runtime` if this given `path` is not a directory
    pub fn add_packet(
        mut self,
        data: &gng_shared::Packet,
        patterns: &[glob::Pattern],
        reproducibility_files: &[std::path::PathBuf],
    ) -> gng_shared::Result<Self> {
        let path = self
            .packet_directory
            .take()
            .unwrap_or(std::env::current_dir()?);

        let p = crate::packager::packet::Packet {
            path,
            data: data.clone(),
            pattern: patterns.to_vec(),
            writer: None,
            reproducibility_files: reproducibility_files.to_vec(),
        };

        packet::validate_packets(&p, &self.packets)?;

        self.packets.push(p);

        Ok(self)
    }

    /// Set up a factory for packet writers.
    #[cfg(tests)]
    pub fn packet_factory(&mut self, factory: Box<PacketWriterFactory>) -> &mut Self {
        self.packet_factory = factory;
        self
    }

    /// Set up a factory for an iterator to get all the files that need to get packaged.
    #[cfg(tests)]
    pub fn iterator_factory(&mut self, factory: Box<PackagingIteratorFactory>) -> &mut Self {
        self.iterator_factory = factory;
        self
    }

    /// Built the actual `Packager`.
    #[must_use]
    pub fn build(self) -> Packager {
        Packager {
            packet_factory: self.packet_factory,
            packets: Some(self.packets),
            iterator_factory: self.iterator_factory,
        }
    }
}

impl Default for PackagerBuilder {
    fn default() -> Self {
        Self {
            packet_directory: None,
            packet_factory: Box::new(|packet_path, packet_name| {
                gng_shared::package::create_packet_writer(packet_path, packet_name)
            }),
            packets: Vec::new(),
            iterator_factory: Box::new(
                |packaging_directory| -> gng_shared::Result<Box<PackagingIterator>> {
                    Ok(Box::new(MimeTypeDirectoryIterator::new(
                        packaging_directory,
                    )?))
                },
            ),
        }
    }
}

// ----------------------------------------------------------------------
// - Packager:
// ----------------------------------------------------------------------

/// A simple Packet creator
pub struct Packager {
    /// The `PacketWriterFactory` to use to create packets
    packet_factory: Box<PacketWriterFactory>,
    /// The actual `Packet` definitions.
    packets: Option<Vec<crate::packager::packet::Packet>>,
    /// The factory used to create the iterator for all files that are to be packaged.
    iterator_factory: Box<PackagingIteratorFactory>,
}

impl Packager {
    /// Package the `base_directory` up into individual Packets.
    ///
    /// # Errors
    /// none yet
    pub fn package(
        &mut self,
        package_directory: &std::path::Path,
    ) -> gng_shared::Result<Vec<std::path::PathBuf>> {
        let package_directory = package_directory.canonicalize()?;

        tracing::debug!("Packaging \"{}\"...", package_directory.to_string_lossy());
        let mut packets = self.packets.take().ok_or(gng_shared::Error::Runtime {
            message: "Packages were already created!".to_string(),
        })?;

        for d in (self.iterator_factory)(&package_directory)? {
            let packet_info = d?;
            if packet_info.on_disk == package_directory {
                continue;
            }

            let packaged_path_str = packet_info.in_packet.path().to_string_lossy().to_string();

            let packet = packets
                .iter_mut()
                .find(|p| p.contains(&packet_info.in_packet, &packet_info.mime_type))
                .ok_or(gng_shared::Error::Runtime {
                    message: format!(
                        "\"{}\" not packaged: no glob pattern matched.",
                        packaged_path_str,
                    ),
                })?;

            tracing::trace!(
                "    [{}] {:?} - {}: [= {}]",
                packet.data.name,
                packet_info.in_packet,
                packet_info.mime_type,
                packet_info.on_disk.to_string_lossy()
            );

            packet.store_path(
                &self.packet_factory,
                &packet_info.in_packet,
                &packet_info.on_disk,
            )?;
        }

        let mut result = Vec::new();
        for p in &mut packets {
            result.append(&mut p.finish()?);
        }
        Ok(result)
    }
}
