// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::{packet::PacketWriter, PacketFileData};

use eyre::WrapErr;

// - Helper:
// ----------------------------------------------------------------------

fn create_packet_meta_data_directory(
    writer: &mut dyn PacketWriter,
    packet_name: &std::ffi::OsStr,
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

    Ok(packet_meta_dir)
}

fn side_facets_from(
    definitions: &[super::NamedFacet],
    packet: &gng_shared::PacketFileData,
) -> eyre::Result<Vec<Facet>> {
    definitions
        .iter()
        .filter_map(|d| {
            if packet.dependencies.contains(&d.packet_hash) {
                None
            } else {
                let patterns = match d
                    .facet
                    .patterns
                    .iter()
                    .map(|s| glob::Pattern::new(&s[..]).wrap_err("Invalid glob pattern in facet."))
                    .collect::<eyre::Result<Vec<_>>>()
                {
                    Ok(p) => p,
                    Err(e) => return Some(Err(e)),
                };

                Some(Ok(Facet {
                    packet_name: packet.name.clone(),
                    packet_version: packet.version.clone(),
                    facet_name: d.name.clone(),
                    facet_definition_packet: d.packet_hash.clone(),
                    mime_types: d.facet.mime_types.clone(),
                    patterns,
                    writer: None,
                    contents_policy: crate::ContentsPolicy::MaybeEmpty,
                }))
            }
        })
        .collect()
}

fn check_contents_policy(
    has_contents: bool,
    contents_policy: &crate::ContentsPolicy,
    full_name: &str,
) -> eyre::Result<()> {
    if has_contents && *contents_policy == crate::ContentsPolicy::Empty {
        Err(eyre::eyre!(
            "\"{}\" contains some data, but it was expected to be empty!",
            full_name
        ))
    } else if !has_contents && *contents_policy == crate::ContentsPolicy::NotEmpty {
        Err(eyre::eyre!(
            "\"{}\" contains no data, but it was expected to have contents!",
            full_name
        ))
    } else {
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - Facet:
// ----------------------------------------------------------------------

pub struct Facet {
    pub packet_name: gng_shared::Name,
    pub packet_version: gng_shared::Version,
    pub facet_name: gng_shared::Name,

    pub facet_definition_packet: gng_shared::Hash,

    pub mime_types: Vec<String>,
    pub patterns: Vec<glob::Pattern>,
    pub contents_policy: crate::ContentsPolicy,

    writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
}

impl std::fmt::Debug for Facet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Facet {{ {}, mime_types: {:?}, patterns: {:?} }}",
            &self.full_debug_name(),
            &self.mime_types,
            &self.patterns,
        )
    }
}

impl Facet {
    #[tracing::instrument(level = "trace")]
    pub fn facets_from(
        definitions: &[super::NamedFacet],
        packet: &gng_shared::PacketFileData,
        contents_policy: crate::ContentsPolicy,
    ) -> eyre::Result<(Vec<Self>, MainFacet)> {
        Ok((
            side_facets_from(definitions, packet)?,
            MainFacet {
                packet: packet.clone(),
                contents_policy,
                writer: None,
            },
        ))
    }

    fn full_debug_name(&self) -> String {
        format!(
            "{}:{}-{}",
            &self.packet_name, &self.facet_name, &self.packet_version,
        )
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
        tracing::info!(
            "\"{}\": Storing {:?}.",
            &self.full_debug_name(),
            &package_path
        );
        let writer = self.get_or_insert_writer(factory)?;
        writer
            .add_path(package_path)
            .wrap_err("Failed to store a path into packet.")
    }

    #[tracing::instrument(level = "trace")]
    pub fn finish(
        &mut self,
    ) -> eyre::Result<Option<(gng_shared::Name, std::path::PathBuf, gng_shared::Hash)>> {
        check_contents_policy(
            self.writer.is_some(),
            &self.contents_policy,
            &self.full_debug_name(),
        )?;

        if self.writer.is_some() {
            let facet_name = self.write_facet_metadata()?;
            let (path, hash) = self.get_writer().expect("Was just is_some()!").finish()?;

            Ok(Some((facet_name, path, hash)))
        } else {
            Ok(None)
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
            self.writer = Some((factory)(
                &self.packet_name,
                &Some(self.facet_name.clone()),
                &self.packet_version,
            )?);
            tracing::info!("Facet file for \"{}\" created", &self.full_debug_name());
        }

        self.get_writer()
    }

    fn write_facet_metadata(&mut self) -> eyre::Result<gng_shared::Name> {
        let packet_name = self.packet_name.clone();
        let packet_version = self.packet_version.clone();
        let facet_name = self.facet_name.clone();

        let writer = self.get_writer()?;
        let meta_data_directory = create_packet_meta_data_directory(
            writer,
            &std::ffi::OsString::from(packet_name.to_string()),
        )?;

        // Have a facet name: Write facet data!
        let data = gng_shared::FacetFileDataBuilder::default()
            .packet_name(packet_name)
            .name(facet_name.clone())
            .version(packet_version)
            .build()?;

        let buffer = serde_json::to_vec(&data).map_err(|e| gng_shared::Error::Conversion {
            expression: "Facet".to_string(),
            typename: "JSON".to_string(),
            message: e.to_string(),
        })?;

        writer
            .add_path(&mut gng_shared::packet::Path::new_file_from_buffer(
                buffer,
                &meta_data_directory,
                &std::ffi::OsString::from(format!("facet-{}.json", &facet_name)),
                0o755,
                0,
                0,
            ))
            .wrap_err("Failed to write facet meta data.")?;

        Ok(facet_name)
    }
}

// ----------------------------------------------------------------------
// - MainFacet:
// ----------------------------------------------------------------------

pub struct MainFacet {
    pub packet: gng_shared::PacketFileData,

    pub contents_policy: crate::ContentsPolicy,

    pub writer: Option<Box<dyn gng_shared::packet::PacketWriter>>,
}

impl std::fmt::Debug for MainFacet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "MainFacet {{ {} }}", &self.packet.name,)
    }
}

impl MainFacet {
    fn full_debug_name(&self) -> String {
        format!("{}-{}", &self.packet.name, &self.packet.version,)
    }

    pub fn store_path(
        &mut self,
        factory: &super::InternalPacketWriterFactory,
        package_path: &mut gng_shared::packet::Path,
        _mime_type: &str,
    ) -> eyre::Result<()> {
        tracing::info!(
            "\"{}\": Storing {:?}.",
            self.full_debug_name(),
            &package_path
        );
        let writer = self.get_or_insert_writer(factory)?;
        writer
            .add_path(package_path)
            .wrap_err("Failed to store a path into packet.")
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
            self.writer = Some((factory)(&self.packet.name, &None, &self.packet.version)?);
            tracing::info!("Packet file for \"{}\" created", &self.full_debug_name());
        }

        self.get_writer()
    }

    #[tracing::instrument(level = "trace", skip(factory))]
    pub fn finish(
        &mut self,
        facets: &[(gng_shared::Name, gng_shared::Hash)],
        factory: &super::InternalPacketWriterFactory,
    ) -> eyre::Result<(PacketFileData, std::path::PathBuf, gng_shared::Hash)> {
        check_contents_policy(
            self.writer.is_some(),
            &self.contents_policy,
            &self.full_debug_name(),
        )?;

        self.get_or_insert_writer(factory)?;

        let data = self.write_facet_metadata(facets)?;
        let (path, hash) = self.get_writer()?.finish()?;

        Ok((data, path, hash))
    }

    fn write_facet_metadata(
        &mut self,
        facets: &[(gng_shared::Name, gng_shared::Hash)],
    ) -> eyre::Result<gng_shared::PacketFileData> {
        let mut data = self.packet.clone();

        let packet_name = self.packet.name.clone();

        let writer = self.get_writer()?;
        let meta_data_directory = create_packet_meta_data_directory(
            writer,
            &std::ffi::OsString::from(packet_name.to_string()),
        )?;

        data.facets.clone_from(
            &facets
                .iter()
                .map(|(n, h)| gng_shared::PacketFacet {
                    name: n.clone(),
                    hash: h.clone(),
                })
                .collect(),
        );

        let buffer = serde_json::to_vec(&data).map_err(|e| gng_shared::Error::Conversion {
            expression: "Packet".to_string(),
            typename: "JSON".to_string(),
            message: e.to_string(),
        })?;

        writer
            .add_path(&mut gng_shared::packet::Path::new_file_from_buffer(
                buffer,
                &meta_data_directory,
                &std::ffi::OsString::from("info.json"),
                0o755,
                0,
                0,
            ))
            .wrap_err("Failed to write facet meta data.")?;

        Ok(data)
    }
}
