// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::packet::{PacketWriter, PacketWriterFactory, Path};
use gng_shared::{Facet, Hash, Name, Packet, Version};

use eyre::WrapErr;

pub mod classifying_directory_iterator;
pub mod deterministic_directory_iterator;
pub mod facet;
pub mod packet;

use classifying_directory_iterator::ClassifyingDirectoryIterator;

// ----------------------------------------------------------------------
// - Types:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct PacketPath {
    in_packet: Path,
    classification: String,
}

type PackagingIteration = eyre::Result<PacketPath>;
type PackagingIterator = dyn Iterator<Item = PackagingIteration>;
type PackagingIteratorFactory = dyn FnMut(&std::path::Path) -> eyre::Result<Box<PackagingIterator>>;

// ----------------------------------------------------------------------
// - NamedFacet:
// ----------------------------------------------------------------------

#[derive(Debug)]
pub struct NamedFacet {
    name: Name,
    packet_hash: Hash,
    facet: Facet,
}

// ----------------------------------------------------------------------
// - PackagerBuilder:
// ----------------------------------------------------------------------

/// A builder for `Packager`
pub struct PackagerBuilder {
    packet_factory_fn: Box<PacketWriterFactory>,
    packets: Vec<crate::packager::packet::PacketBuilder>,
    facet_definitions: Vec<NamedFacet>,
    iterator_factory_fn: Box<PackagingIteratorFactory>,
}

impl PackagerBuilder {
    /// Add a `FacetDefinition`
    ///
    /// # Errors
    /// `gng_shared::Error::Runtime` if this given `facet` is not valid
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_facet(
        mut self,
        name: &Name,
        packet_hash: &Hash,
        facet: &Facet,
    ) -> eyre::Result<Self> {
        let nf = NamedFacet {
            name: name.clone(),
            packet_hash: packet_hash.clone(),
            facet: facet.clone(),
        };
        self.facet_definitions.push(nf);

        Ok(self)
    }

    /// Add a packet
    ///
    /// # Errors
    /// `gng_shared::Error::Runtime` if this given `path` is not a directory
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_packet(
        mut self,
        data: &Packet,
        patterns: &[glob::Pattern],
        contents_policy: crate::ContentsPolicy,
    ) -> eyre::Result<Self> {
        let p =
            crate::packager::packet::PacketBuilder::new(data, patterns.to_vec(), contents_policy);
        packet::validate_packets(&p, &self.packets)?;

        self.packets.push(p);

        Ok(self)
    }

    /// Set up a factory for packet writers.
    pub fn packet_factory(&mut self, factory: Box<PacketWriterFactory>) -> &mut Self {
        self.packet_factory_fn = factory;
        self
    }

    /// Set up a factory for an iterator to get all the files that need to get packaged.
    // #[cfg(tests)]
    pub fn iterator_factory(&mut self, factory: Box<PackagingIteratorFactory>) -> &mut Self {
        self.iterator_factory_fn = factory;
        self
    }

    /// Built the actual `Packager`.
    ///
    /// # Errors
    /// A `gng_shared::Error::Runtime` may be returned when the facets are not valid somehow
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn build(mut self) -> eyre::Result<Packager> {
        let packets = std::mem::take(&mut self.packets);
        let packets = packets
            .into_iter()
            .map(|p| p.build(&self.facet_definitions[..]))
            .collect::<eyre::Result<Vec<_>>>()?;

        Ok(Packager {
            packet_factory: Some(self.packet_factory_fn),
            packets: Some(packets),
            iterator_factory: self.iterator_factory_fn,
        })
    }
}

impl Default for PackagerBuilder {
    fn default() -> Self {
        Self {
            packet_factory_fn: Box::new(|packet_path, packet_name, facet_data, version| {
                gng_shared::packet::create_packet_writer(
                    packet_path,
                    packet_name,
                    facet_data,
                    version,
                )
            }),
            packets: Vec::new(),
            facet_definitions: Vec::new(),
            iterator_factory_fn: Box::new(
                |packaging_directory| -> eyre::Result<Box<PackagingIterator>> {
                    Ok(Box::new(ClassifyingDirectoryIterator::new(
                        packaging_directory,
                    )?))
                },
            ),
        }
    }
}

// ----------------------------------------------------------------------
// - Packager:
// ----------------------------------------------------------------------

/// A type for factories of `PacketWriter`
type InternalPacketWriterFactory =
    Box<dyn Fn(&Name, &Option<(Name, Hash)>, &Version) -> eyre::Result<Box<dyn PacketWriter>>>;

/// A simple Packet creator
pub struct Packager {
    /// The `PacketWriterFactory` to use to create packets
    packet_factory: Option<Box<PacketWriterFactory>>,
    /// The actual `Packet` definitions.
    packets: Option<Vec<crate::packager::packet::Packet>>,
    /// The factory used to create the iterator for all files that are to be packaged.
    iterator_factory: Box<PackagingIteratorFactory>,
}

impl Packager {
    /// Package the `package_directory` up into individual Packets, which will be stored as individual files in the `packet_directory`.
    ///
    /// # Errors
    /// none yet
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn package(
        &mut self,
        package_directory: &std::path::Path,
        packet_directory: &std::path::Path,
    ) -> eyre::Result<Vec<std::path::PathBuf>> {
        let package_directory = package_directory.canonicalize()?;
        let packet_directory = packet_directory.canonicalize()?;

        let factory = std::mem::take(&mut self.packet_factory)
            .ok_or_else(|| eyre::eyre!("Packager has been used up already!"))?;
        let factory: InternalPacketWriterFactory = Box::new(
            move |name, facet_data, version| -> eyre::Result<Box<dyn PacketWriter>> {
                (factory)(&packet_directory, name, facet_data, version)
                    .wrap_err("Failed to create a packet writer.")
            },
        );

        let mut packets = self
            .packets
            .take()
            .ok_or_else(|| eyre::eyre!("Packages were already created!"))?;

        tracing::debug!("Packaging into {:?}", &packets);

        for d in (self.iterator_factory)(&package_directory)? {
            let mut packet_info = d?;
            let in_packet_path = packet_info.in_packet.path();
            if in_packet_path == std::path::PathBuf::from(".") {
                continue;
            }
            if in_packet_path.starts_with("local") {
                return Err(eyre::eyre!(
                    "Trying to package data in admin private area /usr/local"
                ));
            }

            let packaged_path_str = packet_info.in_packet.path().to_string_lossy().to_string();
            let path = packet_info.in_packet.path();

            let packet = packets
                .iter_mut()
                .find(|p| p.contains(&path, &packet_info.classification))
                .ok_or_else(|| {
                    eyre::eyre!(format!(
                        "\"{}\" not packaged: no glob pattern matched.",
                        packaged_path_str,
                    ),)
                })?;

            packet.store_path(
                &factory,
                &mut packet_info.in_packet,
                &packet_info.classification,
            )?;
        }

        let mut result = Vec::new();
        for p in &mut packets {
            result.append(&mut p.finish()?);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gng_shared::{packet::PacketWriter, packet::Path, Hash, Name, Packet, Version};

    use std::convert::{From, TryFrom};

    type PacketContents = Vec<(String, Path)>;
    type SharedPacketHash = std::rc::Rc<std::cell::RefCell<PacketContents>>;

    struct TestPacketWriter {
        result_map: SharedPacketHash,
        packet_info: String,
    }

    struct TestPackagingIterator {
        inputs: Vec<PacketPath>,
        current_pos: usize,
    }

    impl Iterator for TestPackagingIterator {
        type Item = PackagingIteration;

        fn next(&mut self) -> Option<Self::Item> {
            if self.current_pos >= self.inputs.len() {
                None
            } else {
                let item = self.inputs[self.current_pos].clone();
                self.current_pos += 1;
                Some(Ok(item))
            }
        }
    }

    impl PacketWriter for TestPacketWriter {
        fn add_path(&mut self, packet_path: &mut Path) -> gng_shared::Result<()> {
            self.result_map
                .borrow_mut()
                .push((self.packet_info.clone(), packet_path.clone()));
            Ok(())
        }

        fn finish(&mut self) -> gng_shared::Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from(format!(
                "{}.gng",
                self.packet_info
            )))
        }
    }

    fn dir(path: &std::path::Path) -> PacketPath {
        let directory = path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_owned();
        let name = path
            .file_name()
            .unwrap_or(&std::ffi::OsString::new())
            .to_owned();

        PacketPath {
            in_packet: Path::new_directory(&directory, &name, 0o755, 0, 0),
            classification: String::new(),
        }
    }

    fn file(path: &std::path::Path, contents: &[u8]) -> PacketPath {
        let directory = path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_owned();
        let name = path
            .file_name()
            .unwrap_or(&std::ffi::OsString::new())
            .to_owned();

        let mut buffer = Vec::new();
        buffer.extend_from_slice(contents);

        PacketPath {
            in_packet: Path::new_file_from_buffer(buffer, &directory, &name, 0o755, 0, 0),
            classification: String::new(),
        }
    }

    fn packaging_setup(inputs: &[PacketPath]) -> (SharedPacketHash, PackagerBuilder) {
        let result_map = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let result = result_map.clone();

        let mut input_vec = Vec::new();
        input_vec.extend_from_slice(inputs);

        let mut builder = PackagerBuilder::default();
        builder
            .packet_factory(Box::new(
                move |path, name, facet, version| -> gng_shared::Result<Box<dyn PacketWriter>> {
                    Ok(Box::new(TestPacketWriter {
                        result_map: result_map.clone(),
                        packet_info: format!(
                            "{}-{}-{:?}-{}",
                            path.to_string_lossy(),
                            name,
                            facet,
                            version
                        ),
                    }))
                },
            ))
            .iterator_factory(Box::new(move |_| -> eyre::Result<Box<PackagingIterator>> {
                let inputs = std::mem::take(&mut input_vec);

                Ok(Box::new(TestPackagingIterator {
                    inputs,
                    current_pos: 0,
                }))
            }));

        (result, builder)
    }

    #[test]
    fn packager_builder_empty_inputs() {
        let (result_map, mut builder) = packaging_setup(&Vec::new());
        builder = builder
            .add_packet(
                &Packet {
                    source_name: Name::try_from("foo").unwrap(),
                    version: Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    dependencies: Vec::new(),
                    facets: Vec::new(),
                    register_facet: None,
                    hash: Hash::default(),
                },
                &[glob::Pattern::new("**").unwrap()],
                crate::ContentsPolicy::Empty,
            )
            .unwrap();
        let mut packager = builder.build().unwrap();

        let result = packager.package(
            &std::path::PathBuf::from("."),
            &std::path::PathBuf::from("."),
        );

        assert!(result.is_ok());

        let result_map = result_map.replace(Vec::new());
        assert!(result_map.is_empty());
    }

    #[test]
    fn packager_builder_one_input() {
        let (results, mut builder) = packaging_setup(&[dir(std::path::Path::new("."))]);
        builder = builder
            .add_packet(
                &Packet {
                    source_name: Name::try_from("foo").unwrap(),
                    version: Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    dependencies: Vec::new(),
                    facets: Vec::new(),
                    register_facet: None,
                    hash: Hash::default(),
                },
                &[glob::Pattern::new("**").unwrap()],
                crate::ContentsPolicy::NotEmpty,
            )
            .unwrap();
        let mut packager = builder.build().unwrap();

        let result = packager
            .package(
                &std::path::PathBuf::from("."),
                &std::path::PathBuf::from("."),
            )
            .unwrap();

        assert_eq!(result.len(), 1); // One packet was written

        let results = results.replace(Vec::new());
        for d in &results {
            assert!(d.0.ends_with("/gng-build-foo-None-1.0-5"));
            println!("{}: {:?}", d.0, d.1);
        }
        let mut it = results.iter();
        assert_eq!(
            it.next().unwrap().1,
            Path::new_directory(
                std::path::Path::new(""),
                &std::ffi::OsString::from(""),
                0o755,
                0,
                0
            )
        );

        // Metadata
        assert_eq!(
            it.next().unwrap().1,
            Path::new_directory(
                std::path::Path::new(""),
                &std::ffi::OsString::from(".gng"),
                0o755,
                0,
                0
            )
        );
        assert_eq!(
            it.next().unwrap().1,
            Path::new_directory(
                std::path::Path::new(".gng"),
                &std::ffi::OsString::from("foo"),
                0o755,
                0,
                0
            )
        );
        assert_eq!(
            it.next().unwrap().1,
            Path::new_directory(
                std::path::Path::new(".gng/foo"),
                &std::ffi::OsString::from(crate::DEFAULT_FACET_NAME),
                0o755,
                0,
                0
            )
        );
        let meta = &it.next().unwrap().1;
        assert_eq!(
            meta.path(),
            std::path::Path::new(".gng/foo/_MAIN_/info.json")
        );
        assert_eq!(meta.mode(), 0o755);
        assert_eq!(meta.user_id(), 0);
        assert_eq!(meta.group_id(), 0);
        assert_eq!(meta.leaf_type(), "f");

        assert_eq!(it.next(), None);
    }

    #[test]
    fn packager_builder_one_faceted() {
        let (results, mut builder) = packaging_setup(&[
            dir(std::path::Path::new("f1")),
            file(std::path::Path::new("f1/foo"), b"Test FOO"),
            dir(std::path::Path::new("f2")),
            file(std::path::Path::new("f2/bar"), b"Test BAR"),
        ]);
        builder = builder
            .add_packet(
                &Packet {
                    source_name: Name::try_from("foo").unwrap(),
                    version: Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    dependencies: Vec::new(),
                    facets: vec![],
                    register_facet: None,
                    hash: Hash::default(),
                },
                &[glob::Pattern::new("**").unwrap()],
                crate::ContentsPolicy::Empty,
            )
            .unwrap()
            .add_facet(
                &Name::try_from("f1").unwrap(),
                &Hash::default(),
                &Facet {
                    mime_types: vec![],
                    patterns: vec!["f1".to_string(), "f1/**".to_string()],
                },
            )
            .unwrap()
            .add_facet(
                &Name::try_from("unused").unwrap(),
                &Hash::default(),
                &Facet {
                    mime_types: vec![],
                    patterns: vec!["unused".to_string(), "unused/**".to_string()],
                },
            )
            .unwrap()
            .add_facet(
                &Name::try_from("f2").unwrap(),
                &Hash::default(),
                &Facet {
                    mime_types: vec![],
                    patterns: vec!["f2".to_string(), "f2/**".to_string()],
                },
            )
            .unwrap();
        let mut packager = builder.build().unwrap();

        let result = packager
            .package(
                &std::path::PathBuf::from("."),
                &std::path::PathBuf::from("."),
            )
            .unwrap();

        assert_eq!(result.len(), 2);

        let results = results.replace(Vec::new());
        for p in &results {
            let path = p.1.path();

            if path.starts_with(".gng/") {
                continue;
            }
            println!("{}: {:?}", p.0, p.1);

            if path.as_os_str() == std::ffi::OsStr::new("f1")
                || path.as_os_str() == std::ffi::OsStr::new("f1/foo")
            {
                assert!(p.0.contains(r#"Name("f1")"#))
            } else if path.as_os_str() == std::ffi::OsStr::new("f2")
                || path.as_os_str() == std::ffi::OsStr::new("f2/bar")
            {
                assert!(p.0.contains(r#"Name("f2")"#))
            } else {
                assert!(p.0.contains("UNEXPECTED FILE FOUND"));
            }
        }
    }

    #[test]
    fn packager_builder_no_usr_local() {
        let (results, mut builder) = packaging_setup(&[dir(std::path::Path::new("local/foobar"))]);
        builder = builder
            .add_packet(
                &Packet {
                    source_name: Name::try_from("foo").unwrap(),
                    version: Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    dependencies: Vec::new(),
                    facets: Vec::new(),
                    register_facet: None,
                    hash: Hash::default(),
                },
                &[glob::Pattern::new("**").unwrap()],
                crate::ContentsPolicy::NotEmpty,
            )
            .unwrap();
        let mut packager = builder.build().unwrap();

        let result = packager.package(
            &std::path::PathBuf::from("."),
            &std::path::PathBuf::from("."),
        );

        assert!(result.is_err()); // Things in /usr/local should trigger a packaging error!
        let results = results.replace(Vec::new());
        assert_eq!(results.len(), 0);
    }
}
