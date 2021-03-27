// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::package::{PacketWriter, PacketWriterFactory};

// - Helper:
// ----------------------------------------------------------------------

fn same_packet_name(packet: &Packet, packets: &[Packet]) -> bool {
    packets.iter().any(|p| p.path == packet.path)
}

pub fn validate_packets(packet: &Packet, packets: &[Packet]) -> gng_shared::Result<()> {
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

fn create_packet_meta_data_directory(
    writer: &mut dyn PacketWriter,
    packet_name: &std::ffi::OsStr,
) -> gng_shared::Result<std::path::PathBuf> {
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
            &std::ffi::OsString::from(&packet_name),
            0o755,
            0,
            0,
        ),
        &std::path::PathBuf::new(),
    )?;

    Ok(meta_dir.join(packet_name))
}

fn create_packet_meta_data(
    writer: &mut dyn PacketWriter,
    meta_data_directory: &std::path::Path,
    data: &gng_shared::Packet,
) -> gng_shared::Result<()> {
    let buffer = serde_json::to_vec(&data).map_err(|e| gng_shared::Error::Conversion {
        expression: "Packet".to_string(),
        typename: "JSON".to_string(),
        message: e.to_string(),
    })?;

    writer.add_data(
        &gng_shared::package::Path::new_file(
            meta_data_directory,
            &std::ffi::OsString::from("info.json"),
            0o755,
            0,
            0,
            buffer.len() as u64,
        ),
        &buffer,
    )
}

fn create_packet_reproducibility_director(
    writer: &mut dyn PacketWriter,
    meta_data_directory: &std::path::Path,
    reproducibility_data_files: &[std::path::PathBuf],
) -> gng_shared::Result<()> {
    let repro_name = std::ffi::OsString::from("reproducibility");
    writer.add_path(
        &gng_shared::package::Path::new_directory(meta_data_directory, &repro_name, 0o755, 0, 0),
        &std::path::PathBuf::new(),
    )?;

    let repro_dir = meta_data_directory.join(repro_name);
    for repro in reproducibility_data_files {
        let meta = repro.metadata()?;
        let name = repro
            .file_name()
            .ok_or(gng_shared::Error::Runtime {
                message: "Invalid file name given for reproducibility file!".to_string(),
            })?
            .to_owned();

        writer.add_path(
            &gng_shared::package::Path::new_file(&repro_dir, &name, 0o644, 0, 0, meta.len()),
            repro,
        )?;
    }

    Ok(())
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

pub struct Packet {
    pub path: std::path::PathBuf,
    pub data: gng_shared::Packet,
    pub pattern: Vec<glob::Pattern>,
    pub writer: Option<Box<dyn gng_shared::package::PacketWriter>>,
    pub reproducibility_files: Vec<std::path::PathBuf>,
}

impl Packet {
    pub fn contains(&self, packaged_path: &gng_shared::package::Path, _mime_type: &str) -> bool {
        let packaged_path = packaged_path.path();
        self.pattern.iter().any(|p| p.matches_path(&packaged_path))
    }

    pub fn store_path(
        &mut self,
        factory: &PacketWriterFactory,
        packet_path: &gng_shared::package::Path,
        on_disk_path: &std::path::Path,
    ) -> gng_shared::Result<()> {
        let writer = self.get_or_insert_writer(factory)?;
        writer.add_path(packet_path, on_disk_path)
    }

    pub fn finish(&mut self) -> gng_shared::Result<Vec<std::path::PathBuf>> {
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
        let data = std::mem::replace(&mut self.data, gng_shared::Packet::unknown_packet());

        let writer = self.get_writer()?;
        let meta_data_directory = create_packet_meta_data_directory(
            writer,
            &std::ffi::OsString::from(data.name.to_string()),
        )?;

        create_packet_meta_data(writer, &meta_data_directory, &data)?;

        create_packet_reproducibility_director(writer, &meta_data_directory, &reproducibility_files)
    }
}
