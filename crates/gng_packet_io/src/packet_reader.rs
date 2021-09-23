// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! A `PackerReader`

use eyre::{eyre, WrapErr};

use std::io::Read;
use std::{convert::TryFrom, io::Write};

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

/// The packet Metadata
pub type Metadata = Vec<u8>;

type DecompressedReader = zstd::Decoder<'static, std::io::BufReader<std::fs::File>>;
type TarBall = tar::Archive<DecompressedReader>;

fn metadata(
    entries: &mut tar::Entries<'_, DecompressedReader>,
) -> eyre::Result<(std::ffi::OsString, Metadata)> {
    if let Some(meta) = entries.next() {
        let mut entry = meta.wrap_err("Failed to read metadata from packet.")?;
        if !entry.header().entry_type().is_file() {
            return Err(eyre!("Metadata entry of packet must be a file."));
        }
        let path = entry
            .path()
            .wrap_err("Failed to get the path of metadata entry.")?
            .to_path_buf();
        if path.parent() != Some(std::path::Path::new(".gng")) {
            return Err(eyre!(
                "Does not start with metadata entry: Wrong parent directory."
            ));
        }
        if path.extension() != Some(std::ffi::OsStr::new("meta")) {
            return Err(eyre!(
                "Does not start with metadata entry: Wrong file extension."
            ));
        }

        let size = u16::try_from(entry.size()).wrap_err("Metadata was too big.")?;
        let mut metadata = Vec::with_capacity(size as usize);

        entry
            .read_to_end(&mut metadata)
            .wrap_err("Failed to extract metadata.")?;

        if metadata.len() == size as usize {
            Ok((
                path.file_name()
                    .expect("We had an extension earlier")
                    .to_os_string(),
                metadata,
            ))
        } else {
            Err(eyre!("Unexpected metadata size."))
        }
    } else {
        Err(eyre!("Packet was empty."))
    }
}

fn create_tarball(packet_path: &std::path::Path) -> eyre::Result<TarBall> {
    let tarball = std::fs::OpenOptions::new()
        .read(true)
        .open(packet_path)
        .wrap_err(eyre!(
            "Failed to open packet \"{}\" for reading.",
            packet_path.to_string_lossy()
        ))?;
    let tarball = zstd::Decoder::new(tarball).wrap_err(eyre!(
        "Failed to decompress packet \"{}\".",
        &packet_path.to_string_lossy()
    ))?;
    let mut tarball = tar::Archive::new(tarball);

    tarball.set_overwrite(false);
    tarball.set_preserve_mtime(true);
    tarball.set_preserve_permissions(true);

    Ok(tarball)
}

// ----------------------------------------------------------------------
// - PacketReader:
// ----------------------------------------------------------------------

/// Write files and directories into a packet file
pub struct PacketReader {
    packet_path: std::path::PathBuf,
}

impl PacketReader {
    /// Constructor
    #[must_use]
    pub fn new(packet_path: &std::path::Path) -> Self {
        Self {
            packet_path: packet_path.to_path_buf(),
        }
    }

    /// Extract a packet's metadata
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn metadata(&mut self) -> eyre::Result<Vec<u8>> {
        let mut tarball = create_tarball(&self.packet_path)?;
        let mut entries = tarball
            .entries()
            .wrap_err("Failed to read entries from packet.")?;
        let (_, meta_data) = metadata(&mut entries).wrap_err(eyre!(
            "Failed to read metadata from packet \"{}\".",
            self.packet_path.to_string_lossy(),
        ))?;

        Ok(meta_data)
    }

    /// Extract a packet into a usr-directory and returns the meta data
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn extract(&mut self, root_directory: &std::path::Path) -> eyre::Result<Vec<u8>> {
        let mut tarball = create_tarball(&self.packet_path)?;
        let mut entries = tarball
            .entries()
            .wrap_err("Failed to read entries from packet.")?;
        let (meta_file_name, meta_data) = metadata(&mut entries).wrap_err(eyre!(
            "Failed to read metadata from packet \"{}\".",
            self.packet_path.to_string_lossy(),
        ))?;

        let usr_directory = root_directory.join("usr");

        // write metadata:
        let meta_file_path = usr_directory.join(".gng").join(meta_file_name);
        std::fs::File::create(&meta_file_path)
            .wrap_err(eyre!(
                "Failed to open meta data for writing in packet \"{}\".",
                self.packet_path.to_string_lossy()
            ))?
            .write_all(&meta_data)
            .wrap_err(eyre!(
                "Failed to write meta data for packet \"{}\".",
                self.packet_path.to_string_lossy()
            ))?;

        // unpack the other entries:
        for entry in entries {
            let mut entry = entry.wrap_err(eyre!(
                "Failed to read entry from packet \"{}\".",
                self.packet_path.to_string_lossy()
            ))?;

            let packet_path = entry.path()?.to_path_buf();

            entry.unpack_in(&usr_directory).wrap_err(eyre!(
                "Failed to unpack \"{}\" from packet \"{}\".",
                &packet_path.to_string_lossy(),
                self.packet_path.to_string_lossy()
            ))?;
        }

        Ok(meta_data)
    }
}
