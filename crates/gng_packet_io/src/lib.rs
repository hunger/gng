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
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

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

    use std::{convert::TryFrom, io::Read};

    #[test]
    fn integration_packet_io() {
        let tmp = tempfile::Builder::new()
            .prefix("packet-io-")
            .rand_bytes(8)
            .tempdir()
            .expect("Failed to create temporary directory");

        let metadata: Vec<u8> = {
            let mut tmp = Vec::new();
            tmp.extend_from_slice(b"Metadata");
            tmp
        };

        let test_data = b"test data\n";

        // write packet:
        let packet_path = {
            let mut writer = crate::PacketWriter::new(
                tmp.path(),
                &Name::new("packet").unwrap(),
                &None,
                &Version::try_from("1.0").unwrap(),
                metadata.clone(),
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
        };

        let extract_dir = tmp.path().join("extract");
        // extract packet again:
        std::fs::create_dir_all(extract_dir.join("usr/.gng"))
            .expect("Failed to set up extraction directory");

        let mut reader = crate::PacketReader::new(&packet_path);

        // Test metadata extraction only:
        assert_eq!(reader.metadata().expect("Failed to get metadata"), metadata);

        // Test full extraction incl. metadata and all file data.
        assert_eq!(
            reader
                .extract(&extract_dir)
                .expect("Failed to extract packet"),
            metadata
        );

        let mut buf = Vec::new();

        // validate metadata:
        let metadata_file = extract_dir.join("usr/.gng/packet.meta");
        assert!(metadata_file.is_file());
        std::fs::File::open(metadata_file)
            .unwrap()
            .read_to_end(&mut buf)
            .expect("Failed to read metadata from disk");
        assert!(buf == metadata);

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
