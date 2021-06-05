// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object associating a `Name` of a `Packet` with a `Hash` of the `Packet`.

use crate::{Error, Result, Uuid};

use gng_shared::{Hash, Name, PacketFileData};

use std::{
    io::{BufRead, Write},
    ops::Deref,
};

// ----------------------------------------------------------------------
// - Dependency:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
enum Dependency {
    DependsOn(Hash),
    DependedOn(Hash),
    FacetOf(Hash),
    HasFacet(Hash),
}

// ----------------------------------------------------------------------
// - Table:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Table<K, V>
where
    K: serde::de::DeserializeOwned + serde::Serialize + std::cmp::Ord,
    V: serde::de::DeserializeOwned + serde::Serialize + Clone,
{
    table: Vec<(K, V)>,
    must_sort: bool,
    must_save: bool,
}

impl<K, V> Table<K, V>
where
    K: serde::de::DeserializeOwned + serde::Serialize + std::cmp::Ord,
    V: serde::de::DeserializeOwned + serde::Serialize + Clone,
{
    fn load(&mut self, file_name: &std::path::Path) -> Result<()> {
        let file = std::fs::OpenOptions::new().read(true).open(file_name)?;
        let file = std::io::BufReader::new(file);

        self.table.clear();

        for line in file.lines() {
            let line = line.map_err(|e| Error::Packet(format!("Failed to read packets: {}", e)))?;
            let v = serde_json::from_str(&line)
                .map_err(|e| Error::Packet(format!("Failed to parse packets: {}", e)))?;
            self.table.push(v);
        }

        self.table.sort_by(|(k1, v1), (k2, v2)| k1.cmp(k2));

        self.must_save = false;
        self.must_sort = false;

        Ok(())
    }

    fn store(&self, file_name: &std::path::Path) -> Result<bool> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_name)?;
        let mut file = std::io::BufWriter::new(file);

        for v in &self.table {
            file.write_all(
                format!(
                    "{}\n",
                    serde_json::to_string(v).map_err(|e| Error::Packet(format!(
                        "Failed to write packet data: {}",
                        e
                    )))?
                )
                .as_bytes(),
            )?;
        }
        Ok(true)
    }

    fn len(&self) -> usize {
        self.table.len()
    }

    fn find(&self, key: &K) -> &[(K, V)] {
        let start = self.table.partition_point(|(k, v)| k < key);
        let end = self.table.partition_point(|(k, v)| k <= key);

        assert!(start <= end);
        &self.table[start..end]
    }

    fn insert(&mut self, key: K, value: V) {
        let end = self.table.partition_point(|(k, v)| k <= &key);
        self.table.insert(end, (key, value));
    }
}

impl<K, V> Default for Table<K, V>
where
    K: serde::de::DeserializeOwned + serde::Serialize + std::cmp::Ord,
    V: serde::de::DeserializeOwned + serde::Serialize + Clone,
{
    fn default() -> Self {
        Self {
            table: Vec::new(),
            must_sort: false,
            must_save: false,
        }
    }
} // Default for Table

impl<K, V> Deref for Table<K, V>
where
    K: serde::de::DeserializeOwned + serde::Serialize + std::cmp::Ord,
    V: serde::de::DeserializeOwned + serde::Serialize + Clone,
{
    type Target = [(K, V)];

    fn deref(&self) -> &Self::Target {
        &self.table[..]
    }
}

type PacketFileDataTable = Table<Hash, PacketFileData>;
type DependenciesTable = Table<Hash, Dependency>;
type RepositoryNamesTable = Table<(Uuid, Name), Hash>;

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn packet_already_known(
    packet_file_data_table: &PacketFileDataTable,
    packet_hash: &Hash,
    packet_data: &PacketFileData,
) -> crate::Result<bool> {
    let stored = packet_file_data_table.find(packet_hash);
    assert!(stored.len() <= 1);

    if stored.len() > 0 {
        if stored[0].1 == *packet_data {
            return Ok(true); // Same packet already stored!
        }
        return Err(Error::Packet(format!(
            "Packet with hash {} already stored, but with different contents.",
            packet_hash
        )));
    }

    Ok(false)
}

fn validate_packet_to_add(
    dependencies_table: &DependenciesTable,
    names_table: &RepositoryNamesTable,
    packet_hash: &Hash,
    packet_data: &PacketFileData,
) -> crate::Result<()> {
    // TODO: Test for dependency loops
    // TODO: Check for name use
    Ok(())
}

fn add_packet_to_packet_file_data_table(
    packet_file_data_table: &mut PacketFileDataTable,
    packet_hash: Hash,
    packet_data: PacketFileData,
) -> crate::Result<()> {
    packet_file_data_table.insert(packet_hash, packet_data);
    Ok(())
}

fn add_packet_into_dependencies_table(
    dependencies_table: &mut DependenciesTable,
    packet_hash: Hash,
    packet_data: &PacketFileData,
) -> crate::Result<()> {
    for d in &packet_data.dependencies {
        dependencies_table.insert(d.clone(), Dependency::DependedOn(packet_hash.clone()));
        dependencies_table.insert(packet_hash.clone(), Dependency::DependsOn(d.clone()));
    }
    for f in &packet_data.facets {
        let facet_hash = f.hash.clone();
        dependencies_table.insert(facet_hash.clone(), Dependency::FacetOf(packet_hash.clone()));
        dependencies_table.insert(packet_hash.clone(), Dependency::HasFacet(facet_hash));
    }

    Ok(())
}

fn add_packet_into_facet_definition_table(
    facet_definition_table: &mut RepositoryNamesTable,
    repository: Uuid,
    packet_hash: Hash,
    packet_data: &PacketFileData,
) -> crate::Result<()> {
    // Update facet_definition_table
    // if packet_data.register_facet.is_some() {
    //     if let Some(known_definition) =
    //         facet_definition_table.get(&(repository, packet_data.name.clone()))
    //     {
    //         return Err(Error::Packet(format!(
    //             "Facet has already been defined by packet {}.",
    //             known_definition
    //         )));
    //     }
    //     facet_definition_table.set((repository, packet_data.name.clone()), packet_hash);
    // }
    Ok(())
}

fn add_packet_into_names_table(
    names_table: &mut RepositoryNamesTable,
    repository: Uuid,
    packet_hash: Hash,
    packet_name: Name,
) -> crate::Result<()> {
    // if let Some(known_definition) = names_table.get(&(repository, packet_name.clone())) {
    //     return Err(Error::Packet(format!(
    //         "Facet has already been defined by packet {}.",
    //         known_definition
    //     )));
    // }
    // names_table.set((repository, packet_name), packet_hash);
    Ok(())
}

// ----------------------------------------------------------------------
// - PacketDb:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
#[derive(Debug)]
pub struct PacketDb {
    packet_db_directory: Option<std::path::PathBuf>,

    packet_file_data_table: PacketFileDataTable,
    dependencies_table: DependenciesTable,
    facet_definition_table: RepositoryNamesTable,
    names_table: RepositoryNamesTable,

    db_connection: rusqlite::Connection,
}

impl PacketDb {
    const PACKETS_FILE: &'static str = "packets.json";
    const DEPENDENCIES_FILE: &'static str = "dependencies.json";
    const FACETS_FILE: &'static str = "facets.json";
    const NAMES_FILE: &'static str = "names.json";

    /// Open a packet DB with all the packet lists found in `packet_db_directory`.
    ///
    /// # Errors
    /// `Error::Packet` may be returned when some of the packet DB files are invalid in any way
    #[tracing::instrument(level = "trace")]
    pub fn open(packet_db_directory: &std::path::Path) -> Result<Self> {
        Self::create_packet_db(Some(packet_db_directory))
    }

    /// Add a packet into the `PacketDb`
    ///
    /// # Errors
    /// Returns an error if something goes wrong with the database
    pub fn add_packet(
        &mut self,
        repository: &Uuid,
        packet_hash: Hash,
        packet_data: PacketFileData,
    ) -> Result<()> {
        if !packet_already_known(&self.packet_file_data_table, &packet_hash, &packet_data)? {
            // The packet is already in the PAcket DB, so this is a no-op!
            validate_packet_to_add(
                &self.dependencies_table,
                &self.names_table,
                &packet_hash,
                &packet_data,
            )?;
            add_packet_to_packet_file_data_table(
                &mut self.packet_file_data_table,
                packet_hash.clone(),
                packet_data.clone(),
            )?;
            add_packet_into_dependencies_table(
                &mut self.dependencies_table,
                packet_hash.clone(),
                &packet_data,
            )?;
        }

        // Update facet_definition_table
        add_packet_into_facet_definition_table(
            &mut self.facet_definition_table,
            *repository,
            packet_hash.clone(),
            &packet_data,
        )?;
        add_packet_into_names_table(
            &mut self.names_table,
            *repository,
            packet_hash,
            packet_data.name,
        )?;

        Ok(())
    }

    /// list all known `Packet`s in the `PacketDb`
    ///
    /// # Errors
    /// Returns an error if something goes wrong with the database
    #[must_use]
    pub fn all_packets(&self) -> Vec<(Hash, PacketFileData)> {
        self.packet_file_data_table
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    // /// Make sure the repositories known to the packet DB match up with those provided in `repository_uuids`.
    // pub fn sync_repositories(&mut self, repository_uuids: &[Uuid]) {
    //     let old_uuids = self
    //         .repository_packet_db
    //         .keys()
    //         .copied()
    //         .collect::<Vec<_>>();

    //     for u in repository_uuids {
    //         let _res = self
    //             .repository_packet_db
    //             .try_insert(*u, NamePacketsMap::new());
    //     }

    //     let mut to_remove = Vec::new();
    //     for u in self.repository_packet_db.keys() {
    //         if !repository_uuids.contains(u) {
    //             to_remove.push(*u);
    //         }
    //     }

    //     for u in to_remove {
    //         self.repository_packet_db.remove(&u);
    //     }

    //     self.is_modified = self.is_modified
    //         || self
    //             .repository_packet_db
    //             .keys()
    //             .zip(old_uuids.iter())
    //             .any(|(n, o)| n != o);
    // }

    // /// Resolve a `Packet` by its `name`, using a `search_path` of `Repository`s.
    // #[allow(clippy::map_flatten)]
    // #[must_use]
    // pub fn resolve_packet(
    //     &self,
    //     name: &Name,
    //     search_path: &[&Uuid],
    // ) -> Option<(PacketFileData, Hash, Uuid)> {
    //     let mut r = self.resolve_all_packets(name, search_path);
    //     if r.is_empty() {
    //         None
    //     } else {
    //         Some(r.swap_remove(0))
    //     }
    // }

    // /// Resolve a `Packet` by its `name`, using a `search_path` of `Repository`s.
    // #[allow(clippy::map_flatten)]
    // #[must_use]
    // pub fn resolve_all_packets(
    //     &self,
    //     name: &Name,
    //     search_path: &[&Uuid],
    // ) -> Vec<(PacketFileData, Hash, Uuid)> {
    //     search_path
    //         .iter()
    //         .map(|u| {
    //             (
    //                 (self.repository_packet_db.get(*u).map(|pdb| pdb.get(name))).flatten(),
    //                 *u,
    //             )
    //         })
    //         .filter_map(|(p, u)| p.map(|p| (p.packet.clone(), p.hash.clone(), *u)))
    //         .collect()
    // }

    // /// List all `Packet`s in a `Repository`
    // ///
    // /// # Errors
    // /// `Error::Packet` might be returned, if the `Repository` is not known.
    // pub fn list_packets(&self, repository: &Uuid) -> Result<Vec<(&PacketFileData, Hash)>> {
    //     Ok(self
    //         .repository_packet_db
    //         .get(repository)
    //         .ok_or_else(|| Error::Packet(format!("Repository {} not known.", repository)))?
    //         .values()
    //         .map(|pi| (&pi.packet, pi.hash.clone()))
    //         .collect())
    // }

    /// Open a packet DB with all the packet lists found in `packet_db_directory`.
    ///
    /// # Errors
    /// `Error::Packet` may be returned when some of the packet DB files are invalid in any way
    #[tracing::instrument(level = "trace")]
    pub fn open_memory() -> Result<Self> {
        Self::create_packet_db(None)
    }

    #[tracing::instrument(level = "trace")]
    fn create_packet_db(packet_db_directory: Option<&std::path::Path>) -> Result<Self> {
        let mut packet_file_data_table = PacketFileDataTable::default();
        let mut dependencies_table = DependenciesTable::default();
        let mut facet_definition_table = RepositoryNamesTable::default();
        let mut names_table = RepositoryNamesTable::default();

        if let Some(dir) = packet_db_directory {
            packet_file_data_table.load(&dir.join(Self::PACKETS_FILE))?;
            dependencies_table.load(&dir.join(Self::DEPENDENCIES_FILE))?;
            facet_definition_table.load(&dir.join(Self::FACETS_FILE))?;
            names_table.load(&dir.join(Self::NAMES_FILE))?;
        }

        let db_connection = if let Some(dir) = packet_db_directory {
            rusqlite::Connection::open(dir.join("gng.db3"))
                .map_err(|e| Error::Db(format!("Failed to DB file: {}", e)))?
        } else {
            rusqlite::Connection::open_in_memory()
                .map_err(|e| Error::Db(format!("Failed to temporary DB: {}", e)))?
        };

        db_connection.execute()?;

        Ok(Self {
            packet_db_directory: packet_db_directory.map(std::path::Path::to_path_buf),

            packet_file_data_table,
            dependencies_table,
            facet_definition_table,
            names_table,

            db_connection,
        })
    }

    fn store(&self) -> Result<()> {
        if let Some(dir) = &self.packet_db_directory {
            self.packet_file_data_table
                .store(&dir.join(Self::PACKETS_FILE))?;
            self.dependencies_table
                .store(&dir.join(Self::DEPENDENCIES_FILE))?;
            self.facet_definition_table
                .store(&dir.join(Self::FACETS_FILE))?;
            self.names_table.store(&dir.join(Self::NAMES_FILE))?;
        }
        Ok(())
    }
}

impl Drop for PacketDb {
    fn drop(&mut self) {
        let _ignore = self.store(); // ignore errors here!
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

    #[test]
    fn store_packet() {
        let mut packet_db = PacketDb::open_memory().unwrap();

        let uuid = Uuid::new_v4();
        let hash = Hash::sha256("84fd9b0d613af6708cd3e274e61b97d21ad5c61b7568565ec3404759992c0c3d")
            .unwrap();

        packet_db
            .add_packet(&uuid, hash.clone(), create_packet("test"))
            .unwrap();

        let packets = packet_db.all_packets();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].0, hash);
        assert_eq!(packets[0].1.name, Name::try_from("test").unwrap());
    }
}
