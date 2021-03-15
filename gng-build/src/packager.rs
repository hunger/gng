// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use gng_shared::package::{PacketWriter, PacketWriterFactory};

// - Helper:
// ----------------------------------------------------------------------

struct Packet {
    path: std::path::PathBuf,
    pattern: Vec<glob::Pattern>,
    is_optional: bool,
}

fn globs_from_strings(input: &[String]) -> gng_shared::Result<Vec<glob::Pattern>> {
    input
        .iter()
        .map(|s| -> gng_shared::Result<_> {
            glob::Pattern::new(s).map_err(|e| gng_shared::Error::Conversion {
                expression: s.to_string(),
                typename: "glob pattern".to_string(),
                message: e.to_string(),
            })
        })
        .collect::<gng_shared::Result<Vec<glob::Pattern>>>()
}

fn validate_packets(packet: &Packet, packets: &[Packet]) -> gng_shared::Result<()> {
    if packets.iter().find(|p| p.path == packet.path).is_some() {
        return Err(gng_shared::Error::Runtime {
            message: format!(
                "Duplicate packet entry {} found.",
                packet.path.to_string_lossy()
            ),
        });
    }
    Ok(())
}

fn packet_name_from_name_and_suffix(
    name: &gng_shared::Name,
    suffix: &gng_shared::Suffix,
) -> String {
    let name = name.to_string();
    let suffix = suffix.to_string();

    if suffix.is_empty() {
        name
    } else {
        format!("{}-{}", &name, &suffix)
    }
}

//  ----------------------------------------------------------------------
// - PacketeerBuilder:
// ----------------------------------------------------------------------

pub struct PackagerBuilder {
    packet_directory: Option<std::path::PathBuf>,
    base_packet_name: gng_shared::Name,
    base_dir: std::path::PathBuf,
    packet_factory: Box<PacketWriterFactory>,
    packets: Vec<Packet>,
}

impl PackagerBuilder {
    pub fn new_with_factory(
        base_packet_name: &gng_shared::Name,
        base_dir: &std::path::Path,
        factory: Box<PacketWriterFactory>,
    ) -> Self {
        Self {
            packet_directory: None,
            base_packet_name: base_packet_name.to_owned(),
            base_dir: base_dir.to_owned(),
            packet_factory: factory,
            packets: Vec::new(),
        }
    }

    pub fn new(base_packet_name: &gng_shared::Name, base_dir: &std::path::Path) -> Self {
        Self::new_with_factory(
            base_packet_name,
            base_dir,
            Box::new(|packet_path, base_dir| {
                gng_shared::package::create_packet_writer(packet_path, base_dir)
            }),
        )
    }

    pub fn packet_directory(&mut self, path: &std::path::Path) -> gng_shared::Result<&mut Self> {
        if !path.is_dir() {
            return Err(gng_shared::Error::Runtime {
                message: format!(
                    "\"{}\" is not a directory, can not store packets there.",
                    path.to_string_lossy()
                ),
            });
        }

        self.packet_directory = Some(path.to_owned());
        Ok(self)
    }

    fn packet_full_name(
        &self,
        suffix: &gng_shared::Suffix,
    ) -> gng_shared::Result<std::path::PathBuf> {
        let mut path = self
            .packet_directory
            .clone()
            .unwrap_or(std::env::current_dir()?);
        path.push(&packet_name_from_name_and_suffix(
            &self.base_packet_name,
            suffix,
        ));
        Ok(path)
    }

    pub fn add_packet(
        &mut self,
        suffix: &gng_shared::Suffix,
        patterns: &[String],
        is_optional: bool,
    ) -> gng_shared::Result<&mut Self> {
        let mut path = self
            .packet_directory
            .take()
            .unwrap_or(std::env::current_dir()?);
        path.push(&packet_name_from_name_and_suffix(
            &self.base_packet_name,
            suffix,
        ));

        let p = Packet {
            path: self.packet_full_name(suffix)?,
            pattern: globs_from_strings(patterns)?,
            is_optional,
        };

        validate_packets(&p, &self.packets)?;

        self.packets.push(p);

        Ok(self)
    }

    pub fn build(mut self) -> gng_shared::Result<Packager> {
        // Make sure the base package is last! It is fine if this has been added before.
        let _ = self.add_packet(
            &gng_shared::Suffix::new("").expect("This suffix was valid!"),
            &["**/*".to_string()],
            false,
        );

        Ok(Packager {
            base_dir: self.base_dir,
            packet_factory: self.packet_factory,
            packets: self.packets,
        })
    }
}

// ∙----------------------------------------------------------------------
// - Packager:
// ----------------------------------------------------------------------

/// A simple Packet creator
pub struct Packager {
    base_dir: std::path::PathBuf,
    packet_factory: Box<PacketWriterFactory>,
    packets: Vec<Packet>,
}

impl Packager {}

//  ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_generator_ok() {
        assert_eq!(
            packet_name_from_name_and_suffix(
                &gng_shared::Name::new("test").unwrap(),
                &gng_shared::Suffix::new("dev").unwrap()
            ),
            "test-dev".to_string()
        );
        assert_eq!(
            packet_name_from_name_and_suffix(
                &gng_shared::Name::new("test").unwrap(),
                &gng_shared::Suffix::new("").unwrap()
            ),
            "test".to_string()
        );
    }

    #[test]
    fn test_validate_packets_ok() {
        let p = Packet {
            path: std::path::PathBuf::from("/tmp/foo"),
            is_optional: false,
            pattern: globs_from_strings(&["bin/**/*".to_string()]).unwrap(),
        };
        let ps = vec![];
        assert!(validate_packets(&p, &ps).is_ok());
    }
}
