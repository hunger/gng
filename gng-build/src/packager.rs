// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::package::{PacketWriter, PacketWriterFactory};

use std::os::unix::fs::MetadataExt;

// - Helper:
// ----------------------------------------------------------------------

fn globs_from_strings(input: &[String]) -> gng_shared::Result<Vec<glob::Pattern>> {
    input
        .iter()
        .map(|s| {
            glob::Pattern::new(s).map_err(|e| gng_shared::Error::Conversion {
                expression: s.to_string(),
                typename: "glob pattern".to_string(),
                message: e.to_string(),
            })
        })
        .collect::<gng_shared::Result<Vec<glob::Pattern>>>()
}

fn same_packet_name(packet: &Packet, packets: &[Packet]) -> bool {
    packets.iter().any(|p| p.path == packet.path)
}

fn validate_packets(packet: &Packet, packets: &[Packet]) -> gng_shared::Result<()> {
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

fn collect_contents(directory: &std::path::Path) -> gng_shared::Result<Vec<std::fs::DirEntry>> {
    let mut contents = std::fs::read_dir(directory)?
        .map(|i| i.map_err(|e| e.into()))
        .filter(|i| {
            if let Ok(d) = i {
                (d.file_name() != ".") && (d.file_name() != "..")
            } else {
                true
            }
        })
        .collect::<gng_shared::Result<Vec<std::fs::DirEntry>>>()?;
    contents.sort_by_key(std::fs::DirEntry::file_name);
    contents.reverse(); // So that we can pop() in turn later!

    Ok(contents)
}

fn dir_entry_for_path(path: &std::path::Path) -> gng_shared::Result<std::fs::DirEntry> {
    let search_name = path.file_name().ok_or(gng_shared::Error::Runtime {
        message: format!(
            "\"{}\" does not exist: No file name part was found.",
            path.to_string_lossy()
        ),
    })?;

    let parent = path.parent().ok_or(gng_shared::Error::Runtime {
        message: format!(
            "\"{}\" does not exist: Parent is not valid.",
            path.to_string_lossy()
        ),
    })?;
    collect_contents(parent)?
        .into_iter()
        .find(|d| d.file_name() == search_name)
        .ok_or(gng_shared::Error::Runtime {
            message: format!("\"{}\" not found.", path.to_string_lossy()),
        })
}

// ----------------------------------------------------------------------
// - DeterministicDirectoryIterator:
// ----------------------------------------------------------------------

type PackagingIteration = gng_shared::Result<(std::path::PathBuf, gng_shared::package::Path)>;
type PackagingIterator = dyn Iterator<Item = PackagingIteration>;
type PackagingIteratorFactory =
    dyn Fn(&std::path::Path) -> gng_shared::Result<Box<PackagingIterator>>;

struct DeterministicDirectoryIterator {
    stack: Vec<(Vec<std::fs::DirEntry>, std::path::PathBuf)>,
}

impl DeterministicDirectoryIterator {
    fn new(directory: &std::path::Path) -> gng_shared::Result<Self> {
        let base_dir_entry = dir_entry_for_path(directory)?;

        if base_dir_entry.file_type()?.is_dir() {
            Ok(Self {
                stack: vec![(vec![base_dir_entry], std::path::PathBuf::new())],
            })
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!("\"{}\" is not a directory.", directory.to_string_lossy()),
            })
        }
    }

    fn at_end(&self) -> bool {
        self.stack.is_empty()
    }

    fn find_iterator_value(
        &mut self,
    ) -> gng_shared::Result<(std::path::PathBuf, gng_shared::package::Path)> {
        let stack_frame = self.stack.last_mut().expect("Can not be empty!");
        let entry = stack_frame.0.pop().expect("Can not be empty!");
        let directory = stack_frame.1.clone();

        let name = entry.file_name();
        let file_type = entry.file_type()?;
        let meta = entry.path().symlink_metadata()?;
        let mode = meta.mode() & 0o7777_u32;
        let uid = meta.uid();
        let gid = meta.gid();
        let size = meta.size();

        if file_type.is_symlink() {
            let target = entry.path().read_link()?;
            Ok((
                entry.path(),
                gng_shared::package::Path::new_link(&directory, &name, &target, uid, gid),
            ))
        } else if file_type.is_file() {
            Ok((
                entry.path(),
                gng_shared::package::Path::new_file(&directory, &name, mode, uid, gid, size),
            ))
        } else if file_type.is_dir() {
            let contents = collect_contents(&entry.path())?;
            let (new_directory_path, new_directory_name) = if directory.as_os_str().is_empty() {
                (std::path::PathBuf::from("."), std::ffi::OsString::from("."))
            } else {
                (directory.join(&name), name)
            };

            self.stack.push((contents, new_directory_path));

            Ok((
                entry.path(),
                gng_shared::package::Path::new_directory(
                    &directory,
                    &new_directory_name,
                    mode,
                    uid,
                    gid,
                ),
            ))
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!(
                    "Unsupported file type {:?} found in {}.",
                    &file_type,
                    &entry.path().to_string_lossy()
                ),
            })
        }
    }

    fn clean_up(&mut self) {
        loop {
            if let Some(v) = self.stack.last() {
                if v.0.is_empty() {
                    // The top element is empty: pop it and its corresponding directory!
                    self.stack.pop();
                    continue;
                }
            }
            break;
        }
    }
}

impl Iterator for DeterministicDirectoryIterator {
    type Item = PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at_end() {
            return None;
        }

        let result = self.find_iterator_value();
        self.clean_up();

        Some(result)
    }
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

struct Packet {
    path: std::path::PathBuf,
    name: gng_shared::Name,
    pattern: Vec<glob::Pattern>,
    writer: Option<Box<dyn gng_shared::package::PacketWriter>>,
}

impl Packet {
    fn contains(&self, packaged_path: &gng_shared::package::Path) -> bool {
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
        if let Ok(writer) = self.get_writer() {
            let packet_path = writer.finish()?;
            Ok(vec![packet_path])
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!("Packet \"{}\" is empty.", &self.name),
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
            Some((factory)(&self.path, &self.name)?)
        } else {
            None
        };

        if writer.is_some() {
            self.writer = writer;
        }

        self.get_writer()
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
        name: &gng_shared::Name,
        patterns: &[glob::Pattern],
    ) -> gng_shared::Result<Self> {
        let path = self
            .packet_directory
            .take()
            .unwrap_or(std::env::current_dir()?);

        let p = Packet {
            path,
            name: name.clone(),
            pattern: patterns.to_vec(),
            writer: None,
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
                    Ok(Box::new(DeterministicDirectoryIterator::new(
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
            let (fs_path, packaged_path) = d?;
            if fs_path == package_directory {
                continue;
            }

            let packaged_path_str = packaged_path.path().to_string_lossy().to_string();

            let packet = packets
                .iter_mut()
                .find(|p| p.contains(&packaged_path))
                .ok_or(gng_shared::Error::Runtime {
                    message: format!(
                        "\"{}\" not packaged: no glob pattern matched.",
                        packaged_path_str,
                    ),
                })?;

            tracing::trace!(
                "    [{}] {:?}: [= {}]",
                packet.name,
                packaged_path,
                fs_path.to_string_lossy()
            );

            packet.store_path(&self.packet_factory, &packaged_path, &fs_path)?;
        }

        let mut result = Vec::new();
        for p in packets.iter_mut() {
            result.append(&mut p.finish()?);
        }
        Ok(result)
    }
}
