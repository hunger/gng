// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::packet::PacketWriter;

// - Helper:
// ----------------------------------------------------------------------

fn create_packet_meta_data_directory(
    writer: &mut dyn PacketWriter,
    packet_name: &std::ffi::OsStr,
) -> gng_shared::Result<std::path::PathBuf> {
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

    Ok(meta_dir.join(packet_name))
}

fn create_packet_meta_data(
    writer: &mut dyn PacketWriter,
    meta_data_directory: &std::path::Path,
    data: &gng_shared::Packet,
    facet: &Option<gng_shared::Name>,
    description_suffix: &str,
) -> gng_shared::Result<()> {
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

    writer.add_path(&mut gng_shared::packet::Path::new_file_from_buffer(
        buffer,
        meta_data_directory,
        &std::ffi::OsString::from("info.json"),
        0o755,
        0,
        0,
    ))
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
    ) -> gng_shared::Result<Vec<Self>> {
        let mut result = Vec::with_capacity(definitions.len() + 1);
        for d in definitions {
            result.push(Self {
                facet_name: Some(d.name.clone()),
                mime_types: d.mime_types.clone(),
                patterns: d.patterns.clone(),
                data: packet.clone(),
                writer: None,
            });
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
    ) -> gng_shared::Result<()> {
        let writer = self.get_or_insert_writer(factory)?;
        writer.add_path(package_path)
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
        factory: &super::InternalPacketWriterFactory,
    ) -> gng_shared::Result<&mut dyn PacketWriter> {
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

    fn write_packet_metadata(&mut self) -> gng_shared::Result<()> {
        let data = std::mem::replace(&mut self.data, gng_shared::Packet::unknown_packet());

        let writer = self.get_writer()?;
        let meta_data_directory = create_packet_meta_data_directory(
            writer,
            &std::ffi::OsString::from(data.name.to_string()),
        )?;

        create_packet_meta_data(writer, &meta_data_directory, &data, &None, "")
    }
}
