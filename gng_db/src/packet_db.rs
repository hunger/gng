// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object associating a `Name` of a `Packet` with a `Hash` of the `Packet`.

use crate::{Error, Result, Uuid};

use gng_shared::{Hash, Name, PacketFileData};
use std::collections::BTreeMap;

// ----------------------------------------------------------------------
// - PacketInfo:
// ----------------------------------------------------------------------

/// A `Packet` with a `Hash`
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PacketInfo {
    /// The `Packet`.
    #[serde(flatten)]
    packet: PacketFileData,
    /// The `Hash`
    hash: Hash,
}

// - Type aliases:
// ----------------------------------------------------------------------

type NamePacketsMap = BTreeMap<Name, PacketInfo>;
type RepositoryPacketsMap = BTreeMap<Uuid, NamePacketsMap>;

// ----------------------------------------------------------------------
// - PacketDb:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
#[derive(Clone, Debug)]
pub struct PacketDb {
    repository_packet_db: RepositoryPacketsMap,
    is_modified: bool,
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
            is_modified: false,
        })
    }

    /// Persist the `PacketDb` to `packet_db_directory`.
    ///
    /// # Errors
    /// `Error::Packet` may be returned when writing failed
    #[tracing::instrument(level = "trace")]
    pub fn persist(&self, packet_db_directory: &std::path::Path) -> Result<()> {
        if self.is_modified {
            tracing::info!(
                "Persisting Packet DB into {}.",
                packet_db_directory.display()
            );
            backend::write_packet_dbs(packet_db_directory, &self.repository_packet_db)
        } else {
            tracing::info!("Packet DB was not modified. Persisting of DB to storage was skipped.");
            Ok(())
        }
    }

    /// Make sure the repositories known to the packet DB match up with those provided in `repository_uuids`.
    pub fn sync_repositories(&mut self, repository_uuids: &[Uuid]) {
        let old_uuids = self
            .repository_packet_db
            .keys()
            .copied()
            .collect::<Vec<_>>();

        for u in repository_uuids {
            let _res = self
                .repository_packet_db
                .try_insert(*u, NamePacketsMap::new());
        }

        let mut to_remove = Vec::new();
        for u in self.repository_packet_db.keys() {
            if !repository_uuids.contains(u) {
                to_remove.push(*u);
            }
        }

        for u in to_remove {
            self.repository_packet_db.remove(&u);
        }

        self.is_modified = self.is_modified
            || self
                .repository_packet_db
                .keys()
                .zip(old_uuids.iter())
                .any(|(n, o)| n != o);
    }

    /// Resolve a `Packet` by its `name`, using a `search_path` of `Repository`s.
    #[allow(clippy::map_flatten)]
    #[must_use]
    pub fn resolve_packet(
        &self,
        name: &Name,
        search_path: &[&Uuid],
    ) -> Option<(PacketFileData, Hash, Uuid)> {
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
    pub fn resolve_all_packets(
        &self,
        name: &Name,
        search_path: &[&Uuid],
    ) -> Vec<(PacketFileData, Hash, Uuid)> {
        search_path
            .iter()
            .map(|u| {
                (
                    (self.repository_packet_db.get(*u).map(|pdb| pdb.get(name))).flatten(),
                    *u,
                )
            })
            .filter_map(|(p, u)| p.map(|p| (p.packet.clone(), p.hash.clone(), *u)))
            .collect()
    }

    /// List all `Packet`s in a `Repository`
    ///
    /// # Errors
    /// `Error::Packet` might be returned, if the `Repository` is not known.
    pub fn list_packets(&self, repository: &Uuid) -> Result<Vec<(&PacketFileData, Hash)>> {
        Ok(self
            .repository_packet_db
            .get(repository)
            .ok_or_else(|| Error::Packet(format!("Repository {} not known.", repository)))?
            .values()
            .map(|pi| (&pi.packet, pi.hash.clone()))
            .collect())
    }
}

impl Default for PacketDb {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        tracing::info!("Packet DB with packets from 0 repositories created.",);

        Self {
            repository_packet_db: RepositoryPacketsMap::new(),
            is_modified: false,
        }
    }
}

#[allow(clippy::redundant_pub_crate)]
mod backend {
    use super::{NamePacketsMap, RepositoryPacketsMap};
    use crate::{Error, Result, Uuid};

    fn packet_db_constants() -> (String, String) {
        (String::from("packets_"), String::from(".conf"))
    }

    fn uuid_from_packet_file_path(path: &std::path::Path) -> Option<Uuid> {
        let (prefix, suffix) = packet_db_constants();

        let file_name = path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            .to_string_lossy();
        if file_name.starts_with(&prefix) && file_name.ends_with(&suffix) {
            let slice_start = prefix.len();
            let slice_end = file_name.len() - (suffix.len() + 1); // + 1 for the '.'
            match Uuid::parse_str(&file_name[slice_start..slice_end]) {
                Ok(u) => Some(u),
                Err(_) => None,
            }
        } else {
            tracing::trace!("    Skipping {}: Not a packet file.", path.display());
            None
        }
    }

    fn packet_file_path(packet_db_directory: &std::path::Path, uuid: &Uuid) -> std::path::PathBuf {
        let (prefix, suffix) = packet_db_constants();
        packet_db_directory.join(format!("{}{}{}", &prefix, uuid, &suffix))
    }

    fn packet_file_paths(
        packet_db_directory: &std::path::Path,
    ) -> Result<Vec<(std::path::PathBuf, Uuid)>> {
        packet_db_directory
            .read_dir()?
            .filter_map(|rd| {
                rd.map_or_else(
                    |e| Some(Err(Error::Packet(e.to_string()))),
                    |rd| {
                        let path = rd.path();
                        uuid_from_packet_file_path(&path).map(|uuid| Ok((path, uuid)))
                    },
                )
            })
            .collect()
    }

    fn read_packet_file(path: &std::path::Path) -> Result<NamePacketsMap> {
        let file = std::fs::File::open(path)?;
        serde_json::from_reader(std::io::BufReader::new(file)).map_err(|_| {
            Error::Packet(format!(
                "Could not read packet information from {}.",
                &path.display()
            ))
        })
    }

    pub(crate) fn read_packet_dbs(
        packet_db_directory: &std::path::Path,
    ) -> Result<RepositoryPacketsMap> {
        packet_file_paths(packet_db_directory)?
            .iter()
            .map(|(path, uuid)| Ok((*uuid, read_packet_file(path)?)))
            .collect()
    }

    fn write_packet_file(path: &std::path::Path, packets: &NamePacketsMap) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .truncate(true)
            .write(true)
            .open(path)?;
        serde_json::to_writer(std::io::BufWriter::new(file), packets).map_err(|_| {
            Error::Packet(format!(
                "Could not read write information to {}.",
                &path.display()
            ))
        })
    }

    pub(crate) fn write_packet_dbs(
        packet_db_directory: &std::path::Path,
        packet_db: &RepositoryPacketsMap,
    ) -> Result<()> {
        // Find packet files:
        let old_packet_files = packet_file_paths(packet_db_directory)?;

        let written_uuids = packet_db
            .iter()
            .map(|(k, v)| {
                write_packet_file(&packet_file_path(packet_db_directory, k), v)?;
                Ok(*k)
            })
            .collect::<Result<std::collections::HashSet<_>>>()?;

        for (path, _) in old_packet_files
            .iter()
            .filter(|(_, u)| !written_uuids.contains(u))
        {
            std::fs::remove_file(path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gng_shared::Version;

    use std::convert::TryFrom;

    fn create_packet(name: &str) -> PacketFileData {
        PacketFileData {
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
        }
    }
}
