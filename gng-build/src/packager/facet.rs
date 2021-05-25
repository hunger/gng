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
        .unwrap_or_else(|| std::ffi::OsString::from(crate::DEFAULT_FACET_NAME));

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
    data: gng_shared::Packet,
    description_suffix: &str,
) -> eyre::Result<gng_shared::Packet> {
    let mut data = data;
    let ds = description_suffix.to_owned();

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
        .wrap_err("Failed to write packet meta data.")?;

    Ok(data)
}

// ----------------------------------------------------------------------
// - Facet:
// ----------------------------------------------------------------------

pub struct Facet {
    pub facet_data: Option<(gng_shared::Name, gng_shared::Hash)>,
    pub mime_types: Vec<String>,
    pub patterns: Vec<glob::Pattern>,
    pub data: Option<gng_shared::Packet>,
    pub writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
    pub contents_policy: crate::ContentsPolicy,
}

impl std::fmt::Debug for Facet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Facet {{ facet_data: {:?}, mime_types: {:?}, patterns: {:?}, data: {:?} }}",
            &self.facet_data, &self.mime_types, &self.patterns, &self.data,
        )
    }
}

impl Facet {
    #[tracing::instrument(level = "trace")]
    pub fn facets_from(
        definitions: &[super::NamedFacet],
        packet: &gng_shared::Packet,
        contents_policy: crate::ContentsPolicy,
    ) -> eyre::Result<Vec<Self>> {
        let mut result = Vec::with_capacity(definitions.len() + 1);
        for d in definitions {
            if !packet.dependencies.contains(&d.packet_hash) {
                result.push(Self {
                    facet_data: Some((d.name.clone(), d.packet_hash.clone())),
                    mime_types: d.facet.mime_types.clone(),
                    patterns: d
                        .facet
                        .patterns
                        .iter()
                        .map(|s| {
                            glob::Pattern::new(&s[..]).wrap_err("Invalid glob pattern in facet.")
                        })
                        .collect::<eyre::Result<Vec<_>>>()?,
                    data: Some(packet.clone()),
                    writer: None,
                    contents_policy: crate::ContentsPolicy::MaybeEmpty,
                });
            }
        }
        result.push(Self {
            facet_data: None,
            mime_types: Vec::new(),
            patterns: vec![glob::Pattern::new("**").expect("** is a valid pattern")],
            data: Some(packet.clone()),
            writer: None,
            contents_policy,
        });
        Ok(result)
    }

    fn full_debug_name(&self) -> String {
        let data = self.data.as_ref();
        let name = data.map_or("<unknown>".to_owned(), |p| p.name.to_string());
        let facet = self
            .facet_data
            .as_ref()
            .map_or(crate::DEFAULT_FACET_NAME.to_string(), |(n, _)| {
                n.to_string()
            });
        let version = data.map_or("<unknown>".to_owned(), |p| p.version.to_string());
        format!("{}-{}-{}", &name, &facet, &version,)
    }

    #[tracing::instrument(level = "trace")]
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
        let full_name = self.full_debug_name();
        let writer = self.get_or_insert_writer(factory)?;
        tracing::info!("Packet \"{}\": Storing {:?}.", &full_name, &package_path);
        writer
            .add_path(package_path)
            .wrap_err("Failed to store a path into packet.")
    }

    #[tracing::instrument(level = "trace")]
    pub fn finish(
        &mut self,
    ) -> eyre::Result<Vec<(gng_shared::Packet, std::path::PathBuf, gng_shared::Hash)>> {
        let has_contents = self.writer.is_some();

        if has_contents && self.contents_policy == crate::ContentsPolicy::Empty {
            return Err(eyre::eyre!(
                "Facet \"{}\" contains some data, but it was expected to be empty!",
                self.full_debug_name()
            ));
        }
        if !has_contents && self.contents_policy == crate::ContentsPolicy::NotEmpty {
            return Err(eyre::eyre!(
                "Facet \"{}\" contains no data, but it was expected to have contents!",
                self.full_debug_name()
            ));
        }

        if self.writer.is_some() {
            let packet = self.write_packet_metadata()?;
            let (path, hash) = self.get_writer().expect("Was just is_some()!").finish()?;

            Ok(vec![(packet, path, hash)])
        } else {
            Ok(Vec::new())
        }
    }

    fn get_writer(&mut self) -> eyre::Result<&mut dyn PacketWriter> {
        Ok(&mut **(self
            .writer
            .as_mut()
            .ok_or_else(|| eyre::eyre!("No writer found."))?))
    }

    fn get_or_insert_writer(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
    ) -> eyre::Result<&mut dyn PacketWriter> {
        if self.writer.is_none() {
            let data = self
                .data
                .as_ref()
                .ok_or_else(|| eyre::eyre!("No Packet data found: Was this Facet reused?"))?;

            self.writer = Some((factory)(&data.name, &self.facet_data, &data.version)?);
            tracing::info!("Packet file for \"{}\" created", &self.full_debug_name());
        }

        self.get_writer()
    }

    fn write_packet_metadata(&mut self) -> eyre::Result<gng_shared::Packet> {
        let mut data = self
            .data
            .clone()
            .ok_or_else(|| eyre::eyre!("No Packet data found: Was this Facet reused?"))?;

        let mut description_suffix = String::new();

        if let Some((n, h)) = &self.facet_data {
            data.dependencies.push(h.clone()); // Depend on base facet
            description_suffix = n.to_string();
        }

        let facet_name = self
            .facet_data
            .as_ref()
            .map(|(n, _)| std::ffi::OsString::from(n.to_string()));
        let writer = self.get_writer()?;
        let meta_data_directory = create_packet_meta_data_directory(
            writer,
            &std::ffi::OsString::from(data.name.to_string()),
            &facet_name,
        )?;

        create_packet_meta_data(writer, &meta_data_directory, data, &description_suffix)
    }
}
