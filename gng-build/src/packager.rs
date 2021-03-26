// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::package::{PacketWriter, PacketWriterFactory};

pub mod deterministic_directory_iterator;
pub mod mimetype_directory_iterator;

use mimetype_directory_iterator::MimeTypeDirectoryIterator;

// - Helper:
// ----------------------------------------------------------------------

fn same_packet_name(packet: &Packet, packets: &[Packet]) -> bool {
    packets.iter().any(|p| p.path == packet.path)
}

fn validate_packets(packet: &Packet, packets: &[Packet]) -> gng_shared::Result<()> {
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

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

struct Packet {
    path: std::path::PathBuf,
    data: gng_shared::Packet,
    pattern: Vec<glob::Pattern>,
    writer: Option<Box<dyn gng_shared::package::PacketWriter>>,
    reproducibility_files: Vec<std::path::PathBuf>,
}

impl Packet {
    fn contains(&self, packaged_path: &gng_shared::package::Path, _mime_type: &str) -> bool {
        let packaged_path = packaged_path.path();
        self.pattern.iter().any(|p| p.matches_path(&packaged_path))
    }

    fn store_path(
        &mut self,
        factory: &PacketWriterFactory,
        packet_path: &gng_shared::package::Path,
        on_disk_path: &std::path::Path,
    ) -> gng_shared::Result<()> {
        let writer = self.get_or_insert_writer(factory)?;
        writer.add_path(packet_path, on_disk_path)
    }

    fn finish(&mut self) -> gng_shared::Result<Vec<std::path::PathBuf>> {
        if self.writer.is_some() {
            self.write_packet_metadata()?;

            Ok(vec![self
                .get_writer()
                .expect("Was just is_some()!")
                .finish()?])
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!("Packet \"{}\" is empty.", &self.data.name),
            })
        }
    }

    fn get_writer(&mut self) -> gng_shared::Result<&mut dyn PacketWriter> {
        Ok(
            &mut **(self.writer.as_mut().ok_or(gng_shared::Error::Runtime {
                message: "No writer found.".to_string(),
            })?),
        )
    }

    fn get_or_insert_writer(
        &mut self,
        factory: &PacketWriterFactory,
    ) -> gng_shared::Result<&mut dyn PacketWriter> {
        let writer = if self.writer.is_none() {
            Some((factory)(&self.path, &self.data.name)?)
        } else {
            None
        };

        if writer.is_some() {
            self.writer = writer;
        }

        self.get_writer()
    }

    fn write_packet_metadata(&mut self) -> gng_shared::Result<()> {
        let reproducibility_files = std::mem::take(&mut self.reproducibility_files);
        let data = std::mem::replace(
            &mut self.data,
            gng_shared::PacketBuilder::default()
                .try_source_name("unknown")?
                .try_version("unknown")?
                .license("unknown")
                .try_name("unknown")?
                .description("unknown")
                .build()
                .map_err(|e| gng_shared::Error::Runtime {
                    message: format!("Failed to create empty packet: {}", e),
                })?,
        );
        let writer = self.get_writer()?;

        let meta_dir = std::path::PathBuf::from(".gng");

        writer.add_path(
            &gng_shared::package::Path::new_directory(
                &std::path::PathBuf::from("."),
                &meta_dir.as_os_str().to_owned(),
                0o755,
                0,
                0,
            ),
            &std::path::PathBuf::new(),
        )?;

        writer.add_path(
            &gng_shared::package::Path::new_directory(
                &meta_dir,
                &std::ffi::OsString::from(&data.name.to_string()),
                0o755,
                0,
                0,
            ),
            &std::path::PathBuf::new(),
        )?;

        let packet_meta_dir = meta_dir.join(data.name.to_string());

        let buffer = serde_json::to_vec(&data).map_err(|e| gng_shared::Error::Conversion {
            expression: "Packet".to_string(),
            typename: "JSON".to_string(),
            message: e.to_string(),
        })?;

        writer.add_data(
            &gng_shared::package::Path::new_file(
                &packet_meta_dir,
                &std::ffi::OsString::from("info.json"),
                0o755,
                0,
                0,
                buffer.len() as u64,
            ),
            &buffer,
        )?;

        let repro_name = std::ffi::OsString::from("reproducibility");
        writer.add_path(
            &gng_shared::package::Path::new_directory(&packet_meta_dir, &repro_name, 0o755, 0, 0),
            &std::path::PathBuf::new(),
        )?;

        let repro_dir = packet_meta_dir.join(repro_name);
        for repro in reproducibility_files {
            let meta = repro.metadata()?;
            let name = repro.file_name().unwrap_or_default().to_owned();

            if name.is_empty() {
                continue;
            }

            writer.add_path(
                &gng_shared::package::Path::new_file(&repro_dir, &name, 0o644, 0, 0, meta.len()),
                &repro,
            )?;
        }

        Ok(())
    }
}

//  ----------------------------------------------------------------------
// - PackagerBuilder:
// ----------------------------------------------------------------------

/// A builder for `Packager`
pub struct PackagerBuilder {
    packet_directory: Option<std::path::PathBuf>,
    packet_factory: Box<PacketWriterFactory>,
    packets: Vec<Packet>,
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

        let p = Packet {
            path,
            data: data.clone(),
            pattern: patterns.to_vec(),
            writer: None,
            reproducibility_files: reproducibility_files.to_vec(),
        };

        validate_packets(&p, &self.packets)?;

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
    packets: Option<Vec<Packet>>,
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
