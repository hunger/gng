// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! A `PackerReader`

use eyre::{eyre, WrapErr};

use std::io::Read;
use std::{convert::TryFrom, io::Write};

use crate::BinaryPacketDefinition;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

/// The packet Meta data
pub type Metadata = Vec<u8>;

type DecompressedReader = zstd::Decoder<'static, std::io::BufReader<std::fs::File>>;
type TarBall = tar::Archive<DecompressedReader>;

fn extract_metadata(
    entry: &mut tar::Entry<'_, DecompressedReader>,
) -> eyre::Result<(std::ffi::OsString, Metadata)> {
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

    /// Extract a packet's raw meta data
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn raw_metadata(&mut self) -> eyre::Result<Vec<u8>> {
        let mut tarball = create_tarball(&self.packet_path)?;
        let mut entries = tarball
            .entries()
            .wrap_err("Failed to read entries from packet.")?;
        if let Some(entry) = entries.next() {
            let mut entry = entry.wrap_err(eyre!(
                "Failed to extract metadata from packet \"{}\"",
                self.packet_path.to_string_lossy()
            ))?;
            let (_, meta_data) = extract_metadata(&mut entry).wrap_err(eyre!(
                "Failed to read metadata from packet \"{}\".",
                self.packet_path.to_string_lossy(),
            ))?;

            Ok(meta_data)
        } else {
            Err(eyre!(
                "Packet \"{}\" has no metadata.",
                self.packet_path.to_string_lossy()
            ))
        }
    }

    /// Extract a packet's meta data
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn metadata(&mut self) -> eyre::Result<BinaryPacketDefinition> {
        serde_json::from_slice(&self.raw_metadata()?)
            .wrap_err("Failed to deserialize packet meta data")
    }

    /// Generate an overview of packet contents
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn contents(&mut self) -> eyre::Result<(Vec<u8>, Vec<crate::ContentInfo>)> {
        let mut tarball = create_tarball(&self.packet_path)?;
        let entries = tarball
            .entries()
            .wrap_err("Failed to extract entry from packet.")?;
        let mut meta_data = None;
        let mut contents = Vec::new();

        for entry in entries {
            let mut entry = entry.wrap_err(eyre!(
                "Failed to extract metadata from packet \"{}\"",
                self.packet_path.to_string_lossy()
            ))?;

            if meta_data.is_none() {
                let (_, tmp) = extract_metadata(&mut entry)?;
                meta_data = Some(tmp);
            }

            let path = entry
                .path()
                .wrap_err("Failed to extract path")?
                .to_path_buf();
            let mode = entry.header().mode().wrap_err(eyre!(
                "Failed to extract mode of entry \"{}\"",
                &path.to_string_lossy(),
            ))?;
            let user_id = entry.header().uid().wrap_err(eyre!(
                "Failed to extract UID of entry \"{}\"",
                &path.to_string_lossy(),
            ))?;
            let group_id = entry.header().gid().wrap_err(eyre!(
                "Failed to extract GID of entry \"{}\"",
                &path.to_string_lossy(),
            ))?;

            let content_type = if entry.header().entry_type().is_dir() {
                crate::ContentType::Directory {}
            } else if entry.header().entry_type().is_symlink() {
                crate::ContentType::Link {
                    target: entry
                        .link_name()
                        .wrap_err(eyre!(
                            "Failed to extract link name of entry \"{}\"",
                            &path.to_string_lossy(),
                        ))?
                        .unwrap_or_default()
                        .to_path_buf(),
                }
            } else {
                crate::ContentType::File { size: entry.size() }
            };

            contents.push(crate::ContentInfo {
                path,
                mode,
                user_id,
                group_id,
                content_type,
            });
        }

        if let Some(meta_data) = meta_data {
            Ok((meta_data, contents))
        } else {
            Err(eyre!(
                "Packet \"{}\" was empty.",
                self.packet_path.to_string_lossy()
            ))
        }
    }

    /// Extract a packet into a usr-directory and returns the meta data
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn extract(&mut self, root_directory: &std::path::Path) -> eyre::Result<Vec<u8>> {
        let usr_directory = root_directory.join("usr");

        let mut tarball = create_tarball(&self.packet_path)?;
        let entries = tarball
            .entries()
            .wrap_err("Failed to read entries from packet.")?;

        let mut meta_data = None;
        for entry in entries {
            let mut entry = entry.wrap_err(eyre!(
                "Failed to extract entry from packet \"{}\"",
                self.packet_path.to_string_lossy()
            ))?;

            if meta_data.is_none() {
                // read meta data:
                let (meta_file_name, tmp) = extract_metadata(&mut entry).wrap_err(eyre!(
                    "Failed to read metadata from packet \"{}\".",
                    self.packet_path.to_string_lossy(),
                ))?;

                // write meta data:
                let meta_file_path = usr_directory.join(".gng").join(meta_file_name);
                std::fs::File::create(&meta_file_path)
                    .wrap_err(eyre!(
                        "Failed to open meta data for writing in packet \"{}\".",
                        self.packet_path.to_string_lossy()
                    ))?
                    .write_all(&tmp)
                    .wrap_err(eyre!(
                        "Failed to write meta data for packet \"{}\".",
                        self.packet_path.to_string_lossy()
                    ))?;

                meta_data = Some(tmp);
            } else {
                // unpack the other entries:
                let packet_path = entry.path()?.to_path_buf();

                entry.unpack_in(&usr_directory).wrap_err(eyre!(
                    "Failed to unpack \"{}\" from packet \"{}\".",
                    &packet_path.to_string_lossy(),
                    self.packet_path.to_string_lossy()
                ))?;
            }
        }

        if let Some(meta_data) = meta_data {
            Ok(meta_data)
        } else {
            Err(eyre!(
                "Packet \"{}\" has no metadata.",
                self.packet_path.to_string_lossy()
            ))
        }
    }
}
