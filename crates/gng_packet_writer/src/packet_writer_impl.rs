// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! Implementation of `PackerWriter` trait.

use gng_core::{Name, Version};

use eyre::WrapErr;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

type PacketWriter = tar::Builder<zstd::Encoder<'static, std::fs::File>>;

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
fn full_packet_name(packet_name: &Name, facet_data: &Option<Name>, version: &Version) -> String {
    let facet_name_string = match facet_data {
        Some(n) => format!(":{}", n),
        None => String::new(),
    };

    format!("{}{}-{}", packet_name, facet_name_string, version)
}

fn add_directory_raw(
    writer: &mut PacketWriter,
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
    writer: &mut PacketWriter,
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
    writer: &mut PacketWriter,
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
    writer: &mut PacketWriter,
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

// ----------------------------------------------------------------------
// - PacketWriterImplState:
// ----------------------------------------------------------------------

enum PacketWriterImplState {
    Empty {
        full_packet_name: String,
        metadata: Vec<u8>,
    },
    Writing(tar::Builder<zstd::Encoder<'static, std::fs::File>>),
    Done,
}

// ----------------------------------------------------------------------
// - PacketWriterImpl:
// ----------------------------------------------------------------------

/// Write files and directories into a packet file
pub struct PacketWriterImpl {
    full_packet_path: std::path::PathBuf,
    state: PacketWriterImplState,
}

impl PacketWriterImpl {
    pub(crate) fn new(
        packet_path: &std::path::Path,
        packet_name: &Name,
        facet_name: &Option<Name>,
        version: &Version,
        metadata: Vec<u8>,
    ) -> Self {
        // TODO: Make this configurable to support e.g. different compression formats?
        let full_packet_name = full_packet_name(packet_name, facet_name, version);

        let mut full_packet_path = packet_path.join(&full_packet_name);
        full_packet_path.set_extension(&".gng");

        Self {
            full_packet_path,
            state: PacketWriterImplState::Empty {
                full_packet_name,
                metadata,
            },
        }
    }

    fn open_packet_file(
        &mut self,
        func: &dyn Fn(&mut PacketWriter) -> eyre::Result<()>,
    ) -> eyre::Result<()> {
        match &mut self.state {
            PacketWriterImplState::Empty {
                full_packet_name,
                metadata,
            } => {
                let tarball = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&self.full_packet_path)?;
                let tarball = zstd::Encoder::new(tarball, 21)?;

                let mut tarball = tar::Builder::new(tarball);

                add_directory_raw(&mut tarball, std::path::Path::new(".gng"), 0o700, 0, 0)?;

                let metadata_path = std::path::PathBuf::from(".gng").join(&full_packet_name);
                add_buffer_raw(&mut tarball, &metadata_path, metadata, 0x600, 0, 0)?;

                self.state = PacketWriterImplState::Writing(tarball);
                Ok(())
            }
            PacketWriterImplState::Writing(tarball) => func(tarball),
            PacketWriterImplState::Done => Err(eyre::eyre!("Packet file already closed.")),
        }
    }
}

impl crate::PacketWriter for PacketWriterImpl {
    fn add_directory(
        &mut self,
        packet_path: &std::path::Path,
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()> {
        self.open_packet_file(&|writer| {
            add_directory_raw(writer, packet_path, mode, user_id, group_id)
        })
    }

    fn add_buffer(
        &mut self,
        packet_path: &std::path::Path,
        data: &[u8],
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()> {
        self.open_packet_file(&|writer| {
            add_buffer_raw(writer, packet_path, data, mode, user_id, group_id)
        })
    }

    fn add_file(
        &mut self,
        packet_path: &std::path::Path,
        on_disk_path: &std::path::Path,
        size: u64,
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()> {
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

    fn add_link(
        &mut self,
        packet_path: &std::path::Path,
        target_path: &std::path::Path,
    ) -> eyre::Result<()> {
        self.open_packet_file(&|writer| add_link_raw(writer, packet_path, target_path))
    }

    fn finish(&mut self) -> eyre::Result<Option<std::path::PathBuf>> {
        let state = {
            let mut state = PacketWriterImplState::Done;
            std::mem::swap(&mut state, &mut self.state);
            state
        };

        match state {
            PacketWriterImplState::Empty {
                full_packet_name: _,
                metadata: _,
            } => Ok(None),
            PacketWriterImplState::Writing(tarball) => {
                let inner = tarball.into_inner()?;
                inner
                    .finish()
                    .map(|_| Some(self.full_packet_path.clone()))
                    .wrap_err("Failed to finish ZSTD compression")
            }
            PacketWriterImplState::Done => Err(eyre::eyre!("Packet has already been closed.")),
        }
    }
}
