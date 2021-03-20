// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::package::{PacketWriter, PacketWriterFactory};

use std::os::unix::fs::MetadataExt;

// - Helper:
// ----------------------------------------------------------------------

struct Packet {
    path: std::path::PathBuf,
    pattern: Vec<glob::Pattern>,
    is_optional: bool,
}

fn globs_from_strings(input: &[String]) -> gng_shared::Result<Vec<glob::Pattern>> {
    input
        .iter()
        .map(|s| -> gng_shared::Result<_> {
            glob::Pattern::new(s).map_err(|e| gng_shared::Error::Conversion {
                expression: s.to_string(),
                typename: "glob pattern".to_string(),
                message: e.to_string(),
            })
        })
        .collect::<gng_shared::Result<Vec<glob::Pattern>>>()
}

fn same_packet_name(packet: &Packet, packets: &[Packet]) -> bool {
    packets.iter().find(|p| p.path == packet.path).is_some()
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

fn packet_name_from_name_and_suffix(
    name: &gng_shared::Name,
    suffix: &gng_shared::Suffix,
) -> String {
    let name = name.to_string();
    let suffix = suffix.to_string();

    if suffix.is_empty() {
        name
    } else {
        format!("{}-{}", &name, &suffix)
    }
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

//  ----------------------------------------------------------------------
// - PackagerBuilder:
// ----------------------------------------------------------------------

/// A builder for `Packager`
pub struct PackagerBuilder {
    packet_directory: Option<std::path::PathBuf>,
    packet_factory: Box<PacketWriterFactory>,
    packets: Vec<Packet>,
}

impl PackagerBuilder {
    /// Create a new `PackagerBuilder` with a custom factory to create `PacketWriter` with.
    pub fn new_with_factory(factory: Box<PacketWriterFactory>) -> Self {
        Self {
            packet_directory: None,
            packet_factory: factory,
            packets: Vec::new(),
        }
    }

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
        is_optional: bool,
    ) -> gng_shared::Result<Self> {
        let mut path = self
            .packet_directory
            .take()
            .unwrap_or(std::env::current_dir()?);
        path.push(&name.to_string());

        let p = Packet {
            path,
            pattern: patterns.to_vec(),
            is_optional,
        };

        validate_packets(&p, &self.packets)?;

        self.packets.push(p);

        Ok(self)
    }

    /// Built the actual `Packager`.
    #[must_use]
    pub fn build(self) -> Packager {
        Packager {
            packet_factory: self.packet_factory,
            packets: self.packets,
        }
    }
}

impl Default for PackagerBuilder {
    fn default() -> PackagerBuilder {
        PackagerBuilder::new_with_factory(Box::new(|packet_path, base_directory| {
            gng_shared::package::create_packet_writer(packet_path, base_directory)
        }))
    }
}

// ----------------------------------------------------------------------
// - DeterministicDirectoryIterator:
// ----------------------------------------------------------------------

struct DeterministicDirectoryIterator {
    base_directory: std::path::PathBuf,
    contents_stack: Vec<Vec<std::fs::DirEntry>>,
    directory_stack: Vec<gng_shared::package::Dir>,
}

impl DeterministicDirectoryIterator {
    fn new(directory: &std::path::Path) -> gng_shared::Result<Self> {
        let base_dir_entry = dir_entry_for_path(directory)?;

        if base_dir_entry.file_type()?.is_dir() {
            Ok(Self {
                base_directory: directory.to_owned(),
                contents_stack: vec![vec![base_dir_entry]],
                directory_stack: vec![gng_shared::package::Dir {
                    name: std::ffi::OsString::from("usr"),
                    uid: 0,
                    gid: 0,
                    mode: 0o755,
                }],
            })
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!("\"{}\" is not a directory.", directory.to_string_lossy()),
            })
        }
    }

    fn at_end(&self) -> bool {
        self.contents_stack.is_empty()
    }

    fn find_iterator_value(
        &mut self,
    ) -> gng_shared::Result<(std::path::PathBuf, gng_shared::package::Path)> {
        let last_stack_pos = self.contents_stack.len() - 1;
        let last_contents = &mut self.contents_stack[last_stack_pos];
        let entry = last_contents.pop().expect("Can not be empty!");

        let name = entry.file_name();
        let file_type = entry.file_type()?;
        let meta = entry.metadata()?;

        if file_type.is_symlink() {
            Ok((
                entry.path(),
                gng_shared::package::Path {
                    is_absolute: false,
                    directory: self.directory_stack.clone(),
                    leaf: gng_shared::package::PathLeaf::Link {
                        name,
                        target: std::path::PathBuf::new(),
                    },
                },
            ))
        } else if file_type.is_file() {
            Ok((
                entry.path(),
                gng_shared::package::Path {
                    is_absolute: false,
                    directory: self.directory_stack.clone(),
                    leaf: gng_shared::package::PathLeaf::File {
                        name: name.clone(),
                        mode: meta.mode(),
                        uid: meta.uid(),
                        gid: meta.gid(),
                        size: meta.size(),
                    },
                },
            ))
        } else if file_type.is_dir() {
            let this_dir = gng_shared::package::Dir {
                name: name.clone(),
                mode: meta.mode(),
                uid: meta.uid(),
                gid: meta.gid(),
            };
            let mut directory = self.directory_stack.clone();
            let contents = collect_contents(&entry.path())?;
            if contents.is_empty() {
                directory.push(this_dir);
                Ok((
                    entry.path(),
                    gng_shared::package::Path {
                        is_absolute: false,
                        directory,
                        leaf: gng_shared::package::PathLeaf::None,
                    },
                ))
            } else {
                self.directory_stack.push(gng_shared::package::Dir {
                    name: name.clone(),
                    uid: meta.uid(),
                    gid: meta.gid(),
                    mode: meta.mode(),
                });
                self.contents_stack.push(contents);
                self.find_iterator_value()
            }
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
            if let Some(v) = self.contents_stack.last() {
                if v.is_empty() {
                    // The top element is empty: pop it and its corresponding directory!
                    self.contents_stack.pop();
                    self.directory_stack.pop();
                    continue;
                }
            }
            break;
        }
    }
}

impl Iterator for DeterministicDirectoryIterator {
    type Item = gng_shared::Result<(std::path::PathBuf, gng_shared::package::Path)>;

    fn next(&mut self) -> Option<Self::Item> {
        assert_eq!(self.contents_stack.len(), self.directory_stack.len());
        if self.at_end() {
            return None;
        }

        let result = self.find_iterator_value();
        assert_eq!(self.contents_stack.len(), self.directory_stack.len());

        self.clean_up();

        assert_eq!(self.contents_stack.len(), self.directory_stack.len());

        Some(result)
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
    packets: Vec<Packet>,
}

impl Packager {
    /// Package the `base_directory` up into individual Packets.
    ///
    /// # Errors
    /// none yet
    pub fn package(&mut self, package_directory: &std::path::Path) -> gng_shared::Result<()> {
        tracing::debug!("Packaging \"{}\"...", package_directory.to_string_lossy());
        let dit = DeterministicDirectoryIterator::new(package_directory)?;
        for d in dit {
            let (abs_path, tar_path) = d?;

            let path_type = match &tar_path.leaf {
                gng_shared::package::PathLeaf::File {
                    name,
                    mode,
                    uid,
                    gid,
                    size,
                } => "F",
                gng_shared::package::PathLeaf::Link { name, target } => "L",
                gng_shared::package::PathLeaf::None => "D",
            };

            tracing::trace!(
                "    {}: {} [= {}]",
                path_type,
                &tar_path.path().to_string_lossy(),
                abs_path.to_string_lossy()
            );
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_packets_ok() {
        let p = Packet {
            path: std::path::PathBuf::from("/tmp/foo"),
            is_optional: false,
            pattern: globs_from_strings(&["bin/**/*".to_string()]).unwrap(),
        };
        let ps = vec![];
        assert!(validate_packets(&p, &ps).is_ok());
    }
}
