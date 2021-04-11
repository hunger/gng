// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::packet::PacketWriterFactory;

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
    in_packet: gng_shared::packet::Path,
    classification: String,
}

type PackagingIteration = eyre::Result<PacketPath>;
type PackagingIterator = dyn Iterator<Item = PackagingIteration>;
type PackagingIteratorFactory = dyn FnMut(&std::path::Path) -> eyre::Result<Box<PackagingIterator>>;

//  ----------------------------------------------------------------------
// - PackagerBuilder:
// ----------------------------------------------------------------------

/// A builder for `Packager`
pub struct PackagerBuilder {
    packet_factory_fn: Box<PacketWriterFactory>,
    packets: Vec<crate::packager::packet::PacketBuilder>,
    facet_definitions: Vec<gng_shared::Facet>,
    iterator_factory_fn: Box<PackagingIteratorFactory>,
}

impl PackagerBuilder {
    /// Add a `FacetDefinition`
    ///
    /// # Errors
    /// `gng_shared::Error::Runtime` if this given `facet` is not valid
    pub fn add_facet(mut self, facet: gng_shared::Facet) -> eyre::Result<Self> {
        self.facet_definitions.push(facet);

        Ok(self)
    }

    /// Add a packet
    ///
    /// # Errors
    /// `gng_shared::Error::Runtime` if this given `path` is not a directory
    pub fn add_packet(
        mut self,
        data: &gng_shared::Packet,
        patterns: &[glob::Pattern],
    ) -> eyre::Result<Self> {
        let p = crate::packager::packet::PacketBuilder::new(data, patterns.to_vec());
        packet::validate_packets(&p, &self.packets)?;
        self.packets.push(p);

        Ok(self)
    }

    /// Set up a factory for packet writers.
    //     #[cfg(tests)]
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
            packet_factory_fn: Box::new(|packet_path, packet_name, facet_name, version| {
                gng_shared::packet::create_packet_writer(
                    packet_path,
                    packet_name,
                    facet_name,
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
type InternalPacketWriterFactory = Box<
    dyn Fn(
        &gng_shared::Name,
        &Option<gng_shared::Name>,
        &gng_shared::Version,
    ) -> eyre::Result<Box<dyn gng_shared::packet::PacketWriter>>,
>;

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
    pub fn package(
        &mut self,
        package_directory: &std::path::Path,
        packet_directory: &std::path::Path,
    ) -> eyre::Result<Vec<std::path::PathBuf>> {
        let package_directory = package_directory.canonicalize()?;
        let packet_directory = packet_directory.canonicalize()?;

        println!("Packaging");
        let factory =
            std::mem::take(&mut self.packet_factory).ok_or(gng_shared::Error::Runtime {
                message: "Packager has been used up already!".to_string(),
            })?;
        let factory: InternalPacketWriterFactory = Box::new(
            move |name,
                  facet,
                  version|
                  -> eyre::Result<Box<dyn gng_shared::packet::PacketWriter>> {
                (factory)(&packet_directory, name, facet, version)
                    .wrap_err("Failed to create a packet writer.")
            },
        );

        tracing::debug!("Packaging \"{}\"...", package_directory.to_string_lossy());
        let mut packets = self.packets.take().ok_or(gng_shared::Error::Runtime {
            message: "Packages were already created!".to_string(),
        })?;

        tracing::trace!("Building Packets: Setup done.");

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
                .ok_or(gng_shared::Error::Runtime {
                    message: format!(
                        "\"{}\" not packaged: no glob pattern matched.",
                        packaged_path_str,
                    ),
                })?;

            tracing::trace!(
                "    [{}] {:?} - {}",
                packet.data.name,
                packet_info.in_packet,
                packet_info.classification,
            );

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

    use gng_shared::packet::PacketWriter;

    use std::convert::{From, TryFrom};

    type PacketContents = Vec<(String, gng_shared::packet::Path)>;
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

    impl gng_shared::packet::PacketWriter for TestPacketWriter {
        fn add_path(
            &mut self,
            packet_path: &mut gng_shared::packet::Path,
        ) -> gng_shared::Result<()> {
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
            in_packet: gng_shared::packet::Path::new_directory(&directory, &name, 0o755, 0, 0),
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
            in_packet: gng_shared::packet::Path::new_file_from_buffer(
                buffer, &directory, &name, 0o755, 0, 0,
            ),
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
    fn test_packager_builder_empty_inputs() {
        let (result_map, mut builder) = packaging_setup(&Vec::new());
        builder = builder
            .add_packet(
                &gng_shared::Packet {
                    source_name: gng_shared::Name::try_from("foo").unwrap(),
                    version: gng_shared::Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: gng_shared::Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    conflicts: gng_shared::Names::default(),
                    provides: gng_shared::Names::default(),
                    dependencies: gng_shared::Names::default(),
                    facet: None,
                },
                &[glob::Pattern::new("**").unwrap()],
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
    fn test_packager_builder_one_input() {
        let (results, mut builder) = packaging_setup(&[dir(std::path::Path::new("."))]);
        builder = builder
            .add_packet(
                &gng_shared::Packet {
                    source_name: gng_shared::Name::try_from("foo").unwrap(),
                    version: gng_shared::Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: gng_shared::Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    conflicts: gng_shared::Names::default(),
                    provides: gng_shared::Names::default(),
                    dependencies: gng_shared::Names::default(),
                    facet: None,
                },
                &[glob::Pattern::new("**").unwrap()],
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
            gng_shared::packet::Path::new_directory(
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
            gng_shared::packet::Path::new_directory(
                std::path::Path::new(""),
                &std::ffi::OsString::from(".gng"),
                0o755,
                0,
                0
            )
        );
        assert_eq!(
            it.next().unwrap().1,
            gng_shared::packet::Path::new_directory(
                std::path::Path::new(".gng"),
                &std::ffi::OsString::from("foo"),
                0o755,
                0,
                0
            )
        );
        assert_eq!(
            it.next().unwrap().1,
            gng_shared::packet::Path::new_directory(
                std::path::Path::new(".gng/foo"),
                &std::ffi::OsString::from("_MAIN_"),
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
    fn test_packager_builder_one_faceted() {
        let (results, mut builder) = packaging_setup(&[
            dir(std::path::Path::new("f1")),
            file(std::path::Path::new("f1/foo"), b"Test FOO"),
            dir(std::path::Path::new("f2")),
            file(std::path::Path::new("f2/bar"), b"Test BAR"),
        ]);
        builder = builder
            .add_packet(
                &gng_shared::Packet {
                    source_name: gng_shared::Name::try_from("foo").unwrap(),
                    version: gng_shared::Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: gng_shared::Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    conflicts: gng_shared::Names::default(),
                    provides: gng_shared::Names::default(),
                    dependencies: gng_shared::Names::default(),
                    facet: None,
                },
                &[glob::Pattern::new("**").unwrap()],
            )
            .unwrap()
            .add_facet(gng_shared::Facet {
                mime_types: vec![],
                name: gng_shared::Name::try_from("f1").unwrap(),
                patterns: vec!["f1".to_string(), "f1/**".to_string()],
            })
            .unwrap()
            .add_facet(gng_shared::Facet {
                mime_types: vec![],
                name: gng_shared::Name::try_from("unused").unwrap(),
                patterns: vec!["unused".to_string(), "unused/**".to_string()],
            })
            .unwrap()
            .add_facet(gng_shared::Facet {
                mime_types: vec![],
                name: gng_shared::Name::try_from("f2").unwrap(),
                patterns: vec!["f2".to_string(), "f2/**".to_string()],
            })
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
    fn test_packager_builder_no_usr_local() {
        let (results, mut builder) = packaging_setup(&[dir(std::path::Path::new("local/foobar"))]);
        builder = builder
            .add_packet(
                &gng_shared::Packet {
                    source_name: gng_shared::Name::try_from("foo").unwrap(),
                    version: gng_shared::Version::try_from("1.0-5").unwrap(),
                    license: "GPL_v3+".to_string(),
                    name: gng_shared::Name::try_from("foo").unwrap(),
                    description: "The foo packet".to_string(),
                    url: None,
                    bug_url: None,
                    conflicts: gng_shared::Names::default(),
                    provides: gng_shared::Names::default(),
                    dependencies: gng_shared::Names::default(),
                    facet: None,
                },
                &[glob::Pattern::new("**").unwrap()],
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
