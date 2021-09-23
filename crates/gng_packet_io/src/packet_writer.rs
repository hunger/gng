// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! A `PackerWriter`

use gng_core::{Name, Version};

use eyre::{eyre, WrapErr};

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

type TarBall = tar::Builder<zstd::Encoder<'static, std::fs::File>>;

fn create_header(size: u64, mode: u32, user_id: u64, group_id: u64) -> eyre::Result<tar::Header> {
    let mut header = tar::Header::new_gnu();

    {
        let gnu = header
            .as_gnu_mut()
            .expect("Created this as GNU, so should work!");

        gnu.set_atime(0);
        gnu.set_ctime(0);
    }

    header.set_mtime(0);
    header.set_device_major(0)?;
    header.set_device_minor(0)?;
    header.set_size(size);
    header.set_mode(mode);
    header.set_uid(user_id);
    header.set_gid(group_id);

    Ok(header)
}

/// Create the full packet name from the base name.
fn versioned_full_packet_name(
    packet_name: &Name,
    facet_data: &Option<Name>,
    version: &Version,
) -> String {
    let facet_name_string = match facet_data {
        Some(n) => format!(":{}", n),
        None => String::new(),
    };

    format!("{}{}-{}", packet_name, facet_name_string, version)
}

fn add_directory_raw(
    writer: &mut TarBall,
    packet_path: &std::path::Path,
    mode: u32,
    user_id: u64,
    group_id: u64,
) -> eyre::Result<()> {
    let mut header = create_header(0, mode, user_id, group_id)?;
    header.set_entry_type(tar::EntryType::Directory);

    writer
        .append_data(&mut header, &packet_path, std::io::empty())
        .wrap_err("Failed to package a directory.")
}
fn add_buffer_raw(
    writer: &mut TarBall,
    packet_path: &std::path::Path,
    data: &[u8],
    mode: u32,
    user_id: u64,
    group_id: u64,
) -> eyre::Result<()> {
    let mut header = create_header(data.len() as u64, mode, user_id, group_id)?;
    header.set_entry_type(tar::EntryType::Regular);

    writer
        .append_data(&mut header, &packet_path, std::io::Cursor::new(data))
        .wrap_err("Failed to package a buffer.")
}

fn add_file_raw(
    writer: &mut TarBall,
    packet_path: &std::path::Path,
    on_disk_path: &std::path::Path,
    size: u64,
    mode: u32,
    user_id: u64,
    group_id: u64,
) -> eyre::Result<()> {
    let mut header = create_header(size, mode, user_id, group_id)?;
    header.set_entry_type(tar::EntryType::Regular);

    let data = std::fs::OpenOptions::new().read(true).open(&on_disk_path)?;
    let data = std::io::BufReader::new(data);

    writer
        .append_data(&mut header, &packet_path, data)
        .wrap_err("Failed to package a file.")
}

fn add_link_raw(
    writer: &mut TarBall,
    packet_path: &std::path::Path,
    target_path: &std::path::Path,
) -> eyre::Result<()> {
    let mut header = create_header(0, 0o777, 0, 0)?;
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_link_name(&target_path)?;

    writer
        .append_data(&mut header, &packet_path, std::io::empty())
        .wrap_err("Failed to package a symlink.")
}

fn persist(
    full_packet_path: &std::path::Path,
    full_packet_name: &str,
    metadata: &[u8],
) -> eyre::Result<TarBall> {
    let tarball = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(full_packet_path)?;
    let tarball = zstd::Encoder::new(tarball, 21)?;

    let mut tarball = tar::Builder::new(tarball);

    let metadata_path = {
        let mut tmp = std::path::PathBuf::from(".gng").join(&full_packet_name);
        tmp.set_extension("meta");
        tmp
    };
    add_buffer_raw(&mut tarball, &metadata_path, metadata, 0o600, 0, 0)?;

    Ok(tarball)
}

fn close(
    tarball: TarBall,
    full_packet_path: &std::path::Path,
) -> eyre::Result<Option<std::path::PathBuf>> {
    let inner = tarball.into_inner()?;
    inner
        .finish()
        .map(|_| Some(full_packet_path.to_path_buf()))
        .wrap_err("Failed to finish ZSTD compression")
}

// ----------------------------------------------------------------------
// - PacketWriterState:
// ----------------------------------------------------------------------

enum PacketWriterState {
    Empty {
        full_packet_name: String,
        metadata: Vec<u8>,
    },
    Writing(TarBall),
    Done,
}

// ----------------------------------------------------------------------
// - PacketWriter:
// ----------------------------------------------------------------------

/// Write files and directories into a packet file
pub struct PacketWriter {
    full_packet_path: std::path::PathBuf,
    policy: crate::PacketPolicy,
    state: PacketWriterState,
}

impl PacketWriter {
    /// Constructor
    #[must_use]
    pub fn new(
        packet_path: &std::path::Path,
        packet_name: &Name,
        facet_name: &Option<Name>,
        version: &Version,
        metadata: Vec<u8>,
        policy: crate::PacketPolicy,
    ) -> Self {
        // TODO: Make this configurable to support e.g. different compression formats?
        let file_name = versioned_full_packet_name(packet_name, facet_name, version);

        let mut full_packet_path = packet_path.join(&file_name);
        full_packet_path.set_extension(&"gng");

        Self {
            full_packet_path,
            policy,
            state: PacketWriterState::Empty {
                full_packet_name: packet_name.combine(facet_name),
                metadata,
            },
        }
    }

    fn open_packet_file(
        &mut self,
        func: &dyn Fn(&mut TarBall) -> eyre::Result<()>,
    ) -> eyre::Result<()> {
        match &mut self.state {
            PacketWriterState::Empty {
                full_packet_name,
                metadata,
            } => {
                self.state = PacketWriterState::Writing(persist(
                    &self.full_packet_path,
                    full_packet_name,
                    metadata,
                )?);
                self.open_packet_file(func)
            }
            PacketWriterState::Writing(tarball) => func(tarball),
            PacketWriterState::Done => Err(eyre::eyre!("Packet file already closed.")),
        }
    }

    /// Add a directory into the packet.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_directory(
        &mut self,
        packet_path: &std::path::Path,
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()> {
        tracing::debug!(
            "Adding directory \"{}\" to packet \"{}\".",
            packet_path.to_string_lossy(),
            &self.full_packet_path.to_string_lossy(),
        );
        self.open_packet_file(&|writer| {
            add_directory_raw(writer, packet_path, mode, user_id, group_id)
        })
    }

    /// Add a buffer into the packet.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_buffer(
        &mut self,
        packet_path: &std::path::Path,
        data: &[u8],
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()> {
        tracing::debug!(
            "Adding buffer into \"{}\" to packet \"{}\".",
            packet_path.to_string_lossy(),
            &self.full_packet_path.to_string_lossy(),
        );
        self.open_packet_file(&|writer| {
            add_buffer_raw(writer, packet_path, data, mode, user_id, group_id)
        })
    }

    /// Add a file into the packet.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_file(
        &mut self,
        packet_path: &std::path::Path,
        on_disk_path: &std::path::Path,
        size: u64,
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()> {
        tracing::debug!(
            "Adding file as \"{}\" to packet \"{}\".",
            packet_path.to_string_lossy(),
            &self.full_packet_path.to_string_lossy(),
        );
        self.open_packet_file(&|writer| {
            add_file_raw(
                writer,
                packet_path,
                on_disk_path,
                size,
                mode,
                user_id,
                group_id,
            )
        })
    }

    /// Add a link into the packet.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_link(
        &mut self,
        packet_path: &std::path::Path,
        target_path: &std::path::Path,
    ) -> eyre::Result<()> {
        tracing::debug!(
            "Adding link as \"{}\" to packet \"{}\".",
            packet_path.to_string_lossy(),
            &self.full_packet_path.to_string_lossy(),
        );

        self.open_packet_file(&|writer| add_link_raw(writer, packet_path, target_path))
    }

    /// Finish writing a packet.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn finish(&mut self) -> eyre::Result<Option<std::path::PathBuf>> {
        let state = {
            let mut state = PacketWriterState::Done;
            std::mem::swap(&mut state, &mut self.state);
            state
        };

        match state {
            PacketWriterState::Empty {
                full_packet_name: fpn,
                metadata: md,
            } => {
                if matches!(&self.policy, crate::PacketPolicy::MustStayEmpty) {
                    tracing::debug!(
                        "Packet \"{}\" stayed empty as requested!",
                        &self.full_packet_path.to_string_lossy(),
                    );

                    let tb = persist(&self.full_packet_path, &fpn, &md).wrap_err(eyre!(
                        "Failed to persist \"{}\".",
                        self.full_packet_path.to_string_lossy(),
                    ))?;
                    close(tb, &self.full_packet_path)
                } else {
                    tracing::debug!(
                        "Packet \"{}\" stayed empty! SKIPPING",
                        &self.full_packet_path.to_string_lossy(),
                    );

                    if matches!(&self.policy, crate::PacketPolicy::MustHaveContents) {
                        Err(eyre!("Packet \"{}\" stayed empty, but must have contents."))
                    } else {
                        Ok(None)
                    }
                }
            }
            PacketWriterState::Writing(tarball) => {
                tracing::debug!(
                    "Packet \"{}\" is getting flushed to disk.",
                    &self.full_packet_path.to_string_lossy(),
                );

                if matches!(&self.policy, crate::PacketPolicy::MustStayEmpty) {
                    Err(eyre!(
                        "Packet \"{}\" has contents, but should have stayed empty."
                    ))
                } else {
                    close(tarball, &self.full_packet_path)
                }
            }
            PacketWriterState::Done => Err(eyre::eyre!("Packet has already been closed.")),
        }
    }
}
