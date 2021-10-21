// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Write gng packets

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::let_unit_value)]

use gng_core::{Name, Names, Version};

// ----------------------------------------------------------------------
// - Enums:
// ----------------------------------------------------------------------

/// A policy for contents in a packet
pub enum PacketPolicy {
    /// The packet must have contents when packaging is done
    MustHaveContents,
    /// The packet may have contents or might be empty
    MayHaveContents,
    /// The packet must be empty
    MustStayEmpty,
}

// ----------------------------------------------------------------------
// - BinaryFacetUsage:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, Eq, serde::Deserialize, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct BinaryFacetUsage {
    /// The name of the facet that is used
    pub name: Name,
}

// ----------------------------------------------------------------------
// - BinaryFacetDefinition:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, Eq, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct BinaryFacetDefinition {
    /// The mime types (as regexp matching `file` output) that belong into this `Facet`
    #[serde(default)]
    pub mime_types: Vec<String>,
    /// Glob-patterns for `files` to include in this `Facet`
    #[serde(default)]
    pub files: Vec<String>,
    /// This Facet extends an existing facet
    pub extends: Option<Name>,
    /// `true` is this facet *must* stay empty at all times
    pub is_forbidden: bool,
}

impl PartialOrd for BinaryFacetDefinition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BinaryFacetDefinition {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

// ----------------------------------------------------------------------
// - BinaryFacet:
// ----------------------------------------------------------------------

/// A binary facet.
#[derive(Clone, Debug, Eq, serde::Deserialize, serde::Serialize)]
pub enum BinaryFacet {
    /// Define a new binary facet
    Definition(BinaryFacetDefinition),
    /// Use an existing binary facet
    Usage(BinaryFacetUsage),
    /// Its the un-faceted packet
    Main,
}

impl PartialEq for BinaryFacet {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl PartialOrd for BinaryFacet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BinaryFacet {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self, &other) {
            (Self::Main | Self::Definition(_), Self::Main | Self::Definition(_)) => {
                std::cmp::Ordering::Equal
            }
            (Self::Main | Self::Definition(_), Self::Usage(_)) => std::cmp::Ordering::Less,
            (Self::Usage(_), Self::Main | Self::Definition(_)) => std::cmp::Ordering::Greater,
            (Self::Usage(lhs), &Self::Usage(rhs)) => lhs.name.cmp(&rhs.name),
        }
    }
}

// ----------------------------------------------------------------------
// - BinaryPacketDefinition:
// ----------------------------------------------------------------------

/// A definition for `Packet` that should get built
#[derive(Clone, Debug, Eq, serde::Deserialize, serde::Serialize)]
pub struct BinaryPacketDefinition {
    /// The `name` of the Packet.
    pub name: Name,
    /// The `name` of the Facet.
    pub version: Version,
    /// The packet description
    pub description: String,
    /// The packet URL
    pub url: String,
    /// The packet URL
    pub bug_url: String,

    /// The `dependencies` of the (faceted) `Packet`
    #[serde(default)]
    pub dependencies: Names,

    /// The `Facet`
    pub facet: BinaryFacet,
}

impl PartialEq for BinaryPacketDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version && self.facet == other.facet
    }
}

impl PartialOrd for BinaryPacketDefinition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BinaryPacketDefinition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let name_cmp = self.name.cmp(&other.name);
        if name_cmp == std::cmp::Ordering::Equal {
            let facet_cmp = self.facet.cmp(&other.facet);
            if facet_cmp == std::cmp::Ordering::Equal {
                self.version.cmp(&other.version)
            } else {
                facet_cmp
            }
        } else {
            name_cmp
        }
    }
}

// ----------------------------------------------------------------------
// - ContentInfo:
// ----------------------------------------------------------------------

/// The type of content that is reported
#[derive(Debug, PartialEq)]
pub enum ContentType {
    /// A file
    File {
        /// The size of the file
        size: u64,
    },
    /// A directory
    Directory {},
    /// A symbolic link
    Link {
        /// The link target
        target: std::path::PathBuf,
    },
}

/// A piece of Contents of the packet
#[derive(Debug, PartialEq)]
pub struct ContentInfo {
    /// The path
    pub path: std::path::PathBuf,
    /// The mode
    pub mode: u32,
    /// The user id
    pub user_id: u64,
    /// The group id
    pub group_id: u64,
    /// The type of contents
    pub content_type: ContentType,
}

// ----------------------------------------------------------------------
// - Modules:
// ----------------------------------------------------------------------

pub mod packet_reader;
pub mod packet_writer;

// ----------------------------------------------------------------------
// - Exports:
// ----------------------------------------------------------------------

pub use packet_reader::PacketReader;
pub use packet_writer::PacketWriter;

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use gng_core::{Name, Version};

    use std::io::Read;

    use crate::ContentInfo;

    fn create_packet(
        directory: &std::path::Path,
        metadata: Vec<u8>,
        test_data: &[u8],
    ) -> std::path::PathBuf {
        let mut writer = crate::PacketWriter::raw_new(
            directory,
            &Name::new("packet").unwrap(),
            &None,
            &Version::try_from("1.0").unwrap(),
            metadata,
            crate::PacketPolicy::MustHaveContents,
        );
        writer
            .add_directory(std::path::Path::new("foo"), 0o755, 0, 0)
            .expect("Failed to write folder information into packet");
        writer
            .add_buffer(
                std::path::Path::new("foo/test.data"),
                test_data,
                0o644,
                0,
                0,
            )
            .expect("Failed to write data into packet");
        let packet_path = writer.finish().expect("Failed to write packet");
        assert!(packet_path.is_some());
        packet_path.unwrap()
    }

    #[test]
    fn integration_packet_io_metadata() {
        let tmp = tempfile::Builder::new()
            .prefix("packet-io-md-")
            .rand_bytes(8)
            .tempdir()
            .expect("Failed to create temporary directory");

        let meta_data: Vec<u8> = {
            let mut tmp = Vec::new();
            tmp.extend_from_slice(b"Metadata");
            tmp
        };

        let test_data = b"test data\n";

        let packet_path = create_packet(tmp.path(), meta_data.clone(), test_data);

        // Test meta data extraction:
        let mut reader = crate::PacketReader::new(&packet_path);
        assert_eq!(
            reader.raw_metadata().expect("Failed to get metadata"),
            meta_data
        );
    }

    #[test]
    fn integration_packet_io_contents() {
        let tmp = tempfile::Builder::new()
            .prefix("packet-io-md-")
            .rand_bytes(8)
            .tempdir()
            .expect("Failed to create temporary directory");

        let meta_data: Vec<u8> = {
            let mut tmp = Vec::new();
            tmp.extend_from_slice(b"Metadata");
            tmp
        };

        let test_data = b"test data\n";

        let packet_path = create_packet(tmp.path(), meta_data.clone(), test_data);

        // Test meta data extraction:
        let mut reader = crate::PacketReader::new(&packet_path);
        let (actual_meta_data, actual_contents) =
            reader.contents().expect("Failed to get metadata");
        assert_eq!(&actual_meta_data, &meta_data);
        assert_eq!(
            &actual_contents,
            &[
                ContentInfo {
                    path: std::path::PathBuf::from(".gng/packet.meta"),
                    mode: 0o600,
                    group_id: 0,
                    user_id: 0,
                    content_type: crate::ContentType::File { size: 8 }
                },
                ContentInfo {
                    path: std::path::PathBuf::from("foo"),
                    mode: 493,
                    user_id: 0,
                    group_id: 0,
                    content_type: crate::ContentType::Directory {},
                },
                ContentInfo {
                    path: std::path::PathBuf::from("foo/test.data"),
                    mode: 420,
                    user_id: 0,
                    group_id: 0,
                    content_type: crate::ContentType::File { size: 10 }
                }
            ]
        );
    }

    #[test]
    fn integration_packet_io_extract() {
        let tmp = tempfile::Builder::new()
            .prefix("packet-io-extract-")
            .rand_bytes(8)
            .tempdir()
            .expect("Failed to create temporary directory");

        let meta_data: Vec<u8> = {
            let mut tmp = Vec::new();
            tmp.extend_from_slice(b"Metadata");
            tmp
        };

        let test_data = b"test data\n";

        let packet_path = create_packet(tmp.path(), meta_data.clone(), test_data);

        let extract_dir = tmp.path().join("extract");
        // extract packet again:
        std::fs::create_dir_all(extract_dir.join("usr/.gng"))
            .expect("Failed to set up extraction directory");

        // Test full extraction incl. meta data and all file data.
        let mut reader = crate::PacketReader::new(&packet_path);
        assert_eq!(
            reader
                .extract(&extract_dir)
                .expect("Failed to extract packet"),
            meta_data
        );

        let mut buf = Vec::new();

        // validate meta data:
        let metadata_file = extract_dir.join("usr/.gng/packet.meta");
        assert!(metadata_file.is_file());
        std::fs::File::open(metadata_file)
            .unwrap()
            .read_to_end(&mut buf)
            .expect("Failed to read metadata from disk");
        assert!(buf == meta_data);

        // validate actual file contents
        buf.clear();
        let data_file = extract_dir.join("usr/foo/test.data");
        assert!(extract_dir.join("usr/foo").is_dir());
        assert!(data_file.is_file());
        std::fs::File::open(data_file)
            .expect("Failed to read extracted test data")
            .read_to_end(&mut buf)
            .expect("Failed to read data from disk");
        println!("Buffer: \"{:?}\", test_data: \"{:?}\".", &buf, &test_data);
        assert!(buf == test_data);
    }
}
