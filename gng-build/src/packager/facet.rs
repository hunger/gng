// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::packet::PacketWriter;

use eyre::WrapErr;

// - Helper:
// ----------------------------------------------------------------------

fn create_packet_meta_data_directory(
    writer: &mut dyn PacketWriter,
    packet_name: &std::ffi::OsStr,
    facet_name: &Option<std::ffi::OsString>,
) -> eyre::Result<std::path::PathBuf> {
    let meta_dir = std::path::PathBuf::from(".gng");

    writer.add_path(&mut gng_shared::packet::Path::new_directory(
        &std::path::PathBuf::new(),
        &meta_dir.as_os_str().to_owned(),
        0o755,
        0,
        0,
    ))?;

    writer.add_path(&mut gng_shared::packet::Path::new_directory(
        &meta_dir,
        &std::ffi::OsString::from(&packet_name),
        0o755,
        0,
        0,
    ))?;

    let packet_meta_dir = meta_dir.join(packet_name);
    let facet_string = facet_name
        .clone()
        .unwrap_or_else(|| std::ffi::OsString::from("_MAIN_"));

    writer.add_path(&mut gng_shared::packet::Path::new_directory(
        &packet_meta_dir,
        &facet_string,
        0o755,
        0,
        0,
    ))?;

    Ok(packet_meta_dir.join(facet_string))
}

fn create_packet_meta_data(
    writer: &mut dyn PacketWriter,
    meta_data_directory: &std::path::Path,
    data: &gng_shared::Packet,
    facet: &Option<gng_shared::Name>,
    description_suffix: &str,
) -> eyre::Result<()> {
    let mut data = data.clone();
    let mut ds = description_suffix.to_owned();
    if let Some(facet) = facet {
        data.dependencies.push(facet.clone());
        if ds.is_empty() {
            ds = facet.to_string();
        }
    }

    if !ds.is_empty() {
        data.description = format!("{} [{}]", &data.description, ds);
    }

    let buffer = serde_json::to_vec(&data).map_err(|e| gng_shared::Error::Conversion {
        expression: "Packet".to_string(),
        typename: "JSON".to_string(),
        message: e.to_string(),
    })?;

    writer
        .add_path(&mut gng_shared::packet::Path::new_file_from_buffer(
            buffer,
            meta_data_directory,
            &std::ffi::OsString::from("info.json"),
            0o755,
            0,
            0,
        ))
        .wrap_err("Failed to write packet meta data.")
}
// ----------------------------------------------------------------------
// - FacetDefinition:
// ----------------------------------------------------------------------

pub struct FacetDefinition {
    pub name: gng_shared::Name,
    pub mime_types: Vec<String>,
    pub patterns: Vec<glob::Pattern>,
}

// ----------------------------------------------------------------------
// - Facet:
// ----------------------------------------------------------------------

pub struct Facet {
    pub facet_name: Option<gng_shared::Name>,
    pub mime_types: Vec<String>,
    pub patterns: Vec<glob::Pattern>,
    pub data: gng_shared::Packet,
    pub writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
}

impl Facet {
    pub fn facets_from(
        definitions: &[FacetDefinition],
        packet: &gng_shared::Packet,
    ) -> eyre::Result<Vec<Self>> {
        let mut result = Vec::with_capacity(definitions.len() + 1);
        for d in definitions {
            if !packet.dependencies.iter().any(|dep| dep == &d.name) {
                result.push(Self {
                    facet_name: Some(d.name.clone()),
                    mime_types: d.mime_types.clone(),
                    patterns: d.patterns.clone(),
                    data: packet.clone(),
                    writer: None,
                });
            }
        }
        result.push(Self {
            facet_name: None,
            mime_types: Vec::new(),
            patterns: vec![glob::Pattern::new("**").expect("** is a valid pattern")],
            data: packet.clone(),
            writer: None,
        });
        Ok(result)
    }

    pub fn contains(&self, path: &std::path::Path, mime_type: &str) -> bool {
        self.patterns.iter().any(|p| p.matches_path(path))
            || self.mime_types.iter().any(|mt| mt == mime_type)
    }

    pub fn store_path(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
        package_path: &mut gng_shared::packet::Path,
        _mime_type: &str,
    ) -> eyre::Result<()> {
        let writer = self.get_or_insert_writer(factory)?;
        writer
            .add_path(package_path)
            .wrap_err("Failed to store a path into packet.")
    }

    pub fn finish(&mut self) -> eyre::Result<Vec<std::path::PathBuf>> {
        if self.writer.is_some() {
            self.write_packet_metadata()?;

            Ok(vec![self
                .get_writer()
                .expect("Was just is_some()!")
                .finish()?])
        } else {
            Ok(Vec::new())
        }
    }

    fn get_writer(&mut self) -> eyre::Result<&mut dyn PacketWriter> {
        Ok(&mut **(self
            .writer
            .as_mut()
            .ok_or(eyre::eyre!("No writer found."))?))
    }

    fn get_or_insert_writer(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
    ) -> eyre::Result<&mut dyn PacketWriter> {
        let writer = if self.writer.is_none() {
            Some((factory)(
                &self.data.name,
                &self.facet_name,
                &self.data.version,
            )?)
        } else {
            None
        };

        if writer.is_some() {
            self.writer = writer;
        }

        self.get_writer()
    }

    fn write_packet_metadata(&mut self) -> eyre::Result<()> {
        let data = std::mem::replace(&mut self.data, gng_shared::Packet::unknown_packet());

        let facet_name = self
            .facet_name
            .as_ref()
            .map(|n| std::ffi::OsString::from(n.to_string()));
        let writer = self.get_writer()?;
        let meta_data_directory = create_packet_meta_data_directory(
            writer,
            &std::ffi::OsString::from(data.name.to_string()),
            &facet_name,
        )?;

        create_packet_meta_data(writer, &meta_data_directory, &data, &None, "")
    }
}
