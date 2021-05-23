// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object associating a `Name` of a `Packet` with a `Hash` of the `Packet`.

use crate::{Error, Result, Uuid};

use gng_shared::{Name, Packet};
use std::collections::BTreeMap;

// - Type aliases:
// ----------------------------------------------------------------------

type NamePacketsMap = BTreeMap<Name, Packet>;
type RepositoryPacketsMap = BTreeMap<Uuid, NamePacketsMap>;

// ----------------------------------------------------------------------
// - PacketDb:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
#[derive(Clone, Debug)]
pub struct PacketDb {
    repository_packet_db: RepositoryPacketsMap,
}

impl PacketDb {
    /// Open a packet DB with all the packet lists found in `packet_db_directory`.
    ///
    /// # Errors
    /// `Error::Packet` may be returned when some of the packet DB files are invalid in any way
    #[tracing::instrument(level = "trace")]
    pub fn open(packet_db_directory: &std::path::Path) -> Result<Self> {
        let repository_packet_db = backend::read_packet_dbs(packet_db_directory)?;

        tracing::info!(
            "Packet DB with packets from {} repositories created.",
            repository_packet_db.len()
        );

        Ok(Self {
            repository_packet_db,
        })
    }

    /// Reset the `PacketDb`
    pub fn reset_db(&mut self) {
        self.repository_packet_db = RepositoryPacketsMap::new();
    }

    /// Add a new `Repository` to the `PacketDb`
    ///
    /// # Errors
    /// `Error::Packet` might be returned, if the `Repository` is already known.
    pub fn add_repository(&mut self, repository: &Uuid, packets: &[Packet]) -> Result<()> {
        let packet_map = packets
            .iter()
            .map(|p| (p.name.clone(), p.clone()))
            .collect::<NamePacketsMap>();

        self.repository_packet_db
            .try_insert(*repository, packet_map)
            .map_err(|e| Error::Packet(format!("Repository {} already known: {}", repository, e)))
            .map(|_| ())
    }

    /// Remove a `Repository` from the `PacketDB` again.
    ///
    /// # Errors
    /// `Error::Packet` might be returned, if the `Repository` is already known.
    pub fn remove_repository(&mut self, repository: &Uuid) -> Result<()> {
        match self.repository_packet_db.remove(repository) {
            Some(_) => Ok(()),
            None => Err(Error::Db(format!("Repository {} not known", repository))),
        }
    }

    /// Resolve a `Packet` by its `name`, using a `search_path` of `Repository`s.
    #[allow(clippy::map_flatten)]
    #[must_use]
    pub fn resolve_packet(&self, name: &Name, search_path: &[&Uuid]) -> Option<(Packet, Uuid)> {
        let mut r = self.resolve_all_packets(name, search_path);
        if r.is_empty() {
            None
        } else {
            Some(r.swap_remove(0))
        }
    }

    /// Resolve a `Packet` by its `name`, using a `search_path` of `Repository`s.
    #[allow(clippy::map_flatten)]
    #[must_use]
    pub fn resolve_all_packets(&self, name: &Name, search_path: &[&Uuid]) -> Vec<(Packet, Uuid)> {
        search_path
            .iter()
            .map(|u| {
                (
                    (self.repository_packet_db.get(*u).map(|pdb| pdb.get(name))).flatten(),
                    *u,
                )
            })
            .filter_map(|(p, u)| p.map(|p| (p.clone(), *u)))
            .collect()
    }

    /// Add a new `Packet` to the DB.
    /// Returns an `Option<Hash>` which will contain a Hash that is no longer used.
    ///
    /// # Errors
    /// `Error::Packet` might be returned, if the `Repository` is not known.
    pub fn add_packet(&mut self, repository: &Uuid, packet: Packet) -> Result<Option<Packet>> {
        let name = packet.name.clone();

        Ok(self
            .repository_packet_db
            .get_mut(repository)
            .ok_or_else(|| Error::Packet(format!("Repository {} not known.", repository)))?
            .insert(name, packet))
    }

    /// Remove a `Packet` from the DB.
    /// Returns an `Option<Hash>` which will contain a Hash that is no longer used.
    ///
    /// # Errors
    /// `Error::Packet` might be returned, if the `Repository` or the `Packet` is not known.
    pub fn remove_packet(&mut self, repository: &Uuid, name: &Name) -> Result<Packet> {
        self.repository_packet_db
            .get_mut(repository)
            .ok_or_else(|| Error::Packet(format!("Repository {} not known.", repository)))?
            .remove(name)
            .ok_or_else(|| {
                Error::Packet(format!(
                    "Packet {} not found in repository {}.",
                    &name, &repository
                ))
            })
    }

    /// List all `Packet`s in a `Repository`
    ///
    /// # Errors
    /// `Error::Packet` might be returned, if the `Repository` is not known.
    pub fn list_packets(&self, repository: &Uuid) -> Result<Vec<&Packet>> {
        Ok(self
            .repository_packet_db
            .get(repository)
            .ok_or_else(|| Error::Packet(format!("Repository {} not known.", repository)))?
            .values()
            .collect())
    }
}

impl Default for PacketDb {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        tracing::info!("Packet DB with packets from 0 repositories created.",);

        Self {
            repository_packet_db: RepositoryPacketsMap::new(),
        }
    }
}

mod backend {
    use super::RepositoryPacketsMap;
    use crate::{Error, Result, Uuid};

    pub fn read_packet_dbs(packet_db_directory: &std::path::Path) -> Result<RepositoryPacketsMap> {
        let packet_file_prefix = "packets_";
        let packet_file_extension = std::ffi::OsStr::new("conf");
        let mut result = RepositoryPacketsMap::new();
        for f in packet_db_directory.read_dir()? {
            let f_path = f?.path();
            if !f_path.is_file() {
                tracing::trace!("    Skipping {}: Not a file.", f_path.display());
                continue;
            }
            let file_stem = f_path
                .file_stem()
                .unwrap_or(std::ffi::OsStr::new(""))
                .to_string_lossy();
            if !file_stem.starts_with(packet_file_prefix) {
                tracing::trace!("    Skipping {}: Not a packet file.", f_path.display());
                continue;
            }
            if let Some(extension) = f_path.extension() {
                if extension == packet_file_extension {
                    let repo =
                        Uuid::parse_str(&file_stem[packet_file_prefix.len()..]).map_err(|_| {
                            Error::Packet(format!(
                                "Could not extract repository UUID form {}.",
                                f_path.display()
                            ))
                        })?;
                    let file = std::fs::File::open(&f_path)?;

                    let packets =
                        serde_json::from_reader(std::io::BufReader::new(file)).map_err(|e| {
                            println!("Original error: {}.", e);
                            Error::Repository(format!(
                                "Could not read repository from {}.",
                                &f_path.display()
                            ))
                        })?;

                    tracing::trace!("    Read {}.", f_path.display());

                    result.insert(repo, packets);
                    continue;
                }
                tracing::trace!(
                    "    Skipping {}: Extension is not \".{}\".",
                    f_path.display(),
                    &packet_file_extension.to_string_lossy()
                );
                continue;
            }
            tracing::trace!("    Skipping {}: No file extension.", f_path.display());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gng_shared::{Hash, Version};

    use std::convert::TryFrom;

    fn create_packet(name: &str) -> Packet {
        Packet {
            source_name: Name::try_from("source").expect("Name was ok."),
            version: Version::try_from("1.0.0").expect("Name was ok."),
            license: "some license".to_string(),
            name: Name::try_from(name).expect("Name was ok."),
            description: "Description".to_string(),
            url: None,
            bug_url: None,
            dependencies: Vec::new(),
            facets: Vec::new(),
            register_facet: None,
            hash: Hash::try_from(
                "sha256:4459946be75c8fe5bef821596387f1222927e996e2acaa5b50d2222f4d2eddc4",
            )
            .expect("Hash was ok"),
        }
    }

    #[test]
    fn add_remove_repository_ok() {
        let mut db = PacketDb::default();
        assert!(db.repository_packet_db.is_empty());

        let uuid = Uuid::new_v4();
        let unused_uuid = Uuid::new_v4();

        db.add_repository(&uuid, &[create_packet("test1")]).unwrap();
        assert_eq!(db.repository_packet_db.len(), 1);

        assert!(db.add_repository(&uuid, &[create_packet("test2")]).is_err());
        assert!(matches!(
            db.repository_packet_db
                .get(&uuid)
                .expect("Should have worked")
                .get(&Name::try_from("test1").expect("Name was fine")),
            Some(_)
        ));

        assert!(db.remove_repository(&unused_uuid).is_err());
        assert!(db.remove_repository(&uuid).is_ok());
        assert!(db.repository_packet_db.is_empty());

        // Re-add the repo that failed before:
        assert!(db.add_repository(&uuid, &[create_packet("test2")]).is_ok());
        assert_eq!(db.repository_packet_db.len(), 1);

        db.reset_db();
        assert!(db.repository_packet_db.is_empty());
    }
}
