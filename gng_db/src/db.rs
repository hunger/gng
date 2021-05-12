// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Db` with all the data on `Packet`s and related data.

mod backend;
mod definitions;
mod repository_db;

use crate::{Error, Repository, Result, Uuid};

use self::{
    definitions::HashedPackets, repository_db::find_repository_by_uuid,
    repository_db::RepositoryDb, repository_db::RepositoryIntern,
};
use gng_shared::Name;

use std::convert::TryFrom;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn validate_repositories_uniqueness(repositories: &[RepositoryIntern]) -> Result<()> {
    let mut known_names = std::collections::HashSet::new();
    let mut known_uuids = std::collections::HashSet::new();

    for r in repositories {
        let r = r.repository();
        if !known_names.insert(r.name.clone()) {
            return Err(Error::Repository(format!(
                "Repository name \"{}\" is not unique.",
                &r.name
            )));
        }
        if !known_uuids.insert(r.uuid) {
            return Err(Error::Repository(format!(
                "Repository UUID \"{}\" is not unique.",
                &r.name
            )));
        }
    }

    Ok(())
}

fn validate_url(url: &str) -> Result<bool> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(false)
    } else if url.starts_with("file://") {
        Ok(true)
    } else {
        Err(Error::Repository(format!("URL \"{}\" is not valid.", url)))
    }
}

fn validate_remote_repository(name: &Name, repository: &crate::RemoteRepository) -> Result<()> {
    if validate_url(&repository.remote_url)? {
        Err(Error::Repository(format!(
            "The remote repository \"{}\" must have a http(s):-url as remote_url.",
            &name
        )))
    } else {
        Ok(())
    }
}

fn validate_repositories_urls_and_sources(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories {
        let r = r.repository();
        if let crate::RepositorySource::Remote(rr) = &r.source {
            validate_remote_repository(&r.name, rr)?;
        }
    }
    Ok(())
}

fn validate_repositories_relations(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories {
        let r = r.repository();
        match &r.relation {
            crate::RepositoryRelation::Dependency(dependencies) => {
                for d in dependencies {
                    if find_repository_by_uuid(repositories, d).is_none() {
                        return Err(Error::Repository(format!(
                            "Repository \"{}\" has unknown dependency \"{}\".",
                            &r.name, &d
                        )));
                    }
                }
            }
            crate::RepositoryRelation::Override(u) => {
                if find_repository_by_uuid(repositories, u).is_none() {
                    return Err(Error::Repository(format!(
                        "Repository \"{}\" overrides unknown repository \"{}\".",
                        &r.name, &u
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_repositories(repositories: &[RepositoryIntern]) -> Result<()> {
    validate_repositories_uniqueness(repositories)?;
    validate_repositories_urls_and_sources(repositories)?;
    validate_repositories_relations(repositories)?;

    Ok(())
}

// ----------------------------------------------------------------------
// - Db:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
pub trait Db {
    // Repository management:

    /// Resolve a user provided repository to a `Uuid`
    fn resolve_repository(&self, input: &str) -> Option<Uuid>;

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn list_repositories(&self) -> Vec<Repository>;

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn add_repository(&mut self, repository_data: Repository) -> Result<()>;

    /// Remove a repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn remove_repository(&mut self, name: &Uuid) -> Result<()>;

    // Packet management:

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    // fn adopt_packet(&mut self, packet: PacketData, repository: &Uuid) -> Result<()>;

    // Debug things:

    /// Run sanity checks on Repository
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn fsck(&self) -> Result<bool>;
}

// ----------------------------------------------------------------------
// - DbImpl:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct DbImpl {
    db_directory: Option<std::path::PathBuf>,

    repository_db: self::repository_db::RepositoryDb,
    hashed_packets: HashedPackets,
}

impl DbImpl {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn new(db_directory: &std::path::Path) -> Result<Self> {
        self::backend::init_db(db_directory)?;

        let (repositories, hashed_packets) = backend::read_db(db_directory)?;

        Ok(Self {
            db_directory: Some(db_directory.to_owned()),

            repository_db: RepositoryDb::new(&repositories[..])?,
            hashed_packets,
        })
    }

    //     // Return a tuple with the PacketIntern and an Optional facet name to register!
    //     #[tracing::instrument(level = "trace")]
    //     fn validate_packet_to_adopt(
    //         &mut self,
    //         packet: &PacketData,
    //         repository: &Uuid,
    //     ) -> Result<(PacketIntern, Option<Name>, Option<Hash>)> {
    //         // TODO: Check for cyclic dependencies in packets or facets

    //         let packet_name = &packet.data.name;
    //         let repository = definitions::find_repository_by_uuid(&self.repositories, repository)
    //             .ok_or_else(|| {
    //                 Error::Repository(format!("Repository \"{}\" not found.", repository))
    //             })?;

    //         // Are we adopting into a local repository?
    //         if !repository.is_local() {
    //             return Err(Error::Packet(
    //                 "Can not adopt a Packet into a remote repository.".to_string(),
    //             ));
    //         }

    //         // TODO: Check for duplicate packet names (using search path!)
    //         // It is OK to have a dupe in a override directory of the current repo!

    //         // Check facet name.
    //         // let facet_name = valid_facet_name(packet, repository, &dependency_group)?;

    //         // let packet_intern = PacketIntern::new(
    //         //     packet.data.clone(),
    //         //     packet.facets.clone(),
    //         //     &dependency_group,
    //         // )?;

    //         let old_hash = repository.packets().get(packet_name).cloned();

    //         // Ok((packet_intern, facet_name, old_hash))
    //         todo!()
    //     }

    //     fn add_hashed_packet(&mut self, hash: &Hash, data: PacketIntern) -> Result<()> {
    //         // if let Some(old_hash) = data.replaces {}
    //         self.hashed_packets.insert(hash.clone(), data);
    //         todo!()
    //     }
}

impl Default for DbImpl {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        Self {
            db_directory: None,
            repository_db: RepositoryDb::default(),
            hashed_packets: HashedPackets::new(),
        }
    }
} // Default for DbImpl

impl Db for DbImpl {
    fn resolve_repository(&self, input: &str) -> Option<Uuid> {
        self.repository_db.resolve_repository(input)
    }

    fn list_repositories(&self) -> Vec<Repository> {
        self.repository_db.list_repositories()
    }

    fn add_repository(&mut self, repository_data: Repository) -> Result<()> {
        self.repository_db.add_repository(repository_data)
    }

    fn remove_repository(&mut self, uuid: &Uuid) -> Result<()> {
        self.repository_db.remove_repository(uuid)
    }

    // #[tracing::instrument(level = "trace", skip(self))]
    // fn adopt_packet(&mut self, packet: PacketData, repository: &Uuid) -> Result<()> {
    //     let (packet_intern, facet_name, old_packet_hash) =
    //         self.validate_packet_to_adopt(&packet, repository)?;

    //     // self.fix_reverse_dependencies(
    //     //     &old_packet_hash,
    //     //     &Some(packet.hash.clone()),
    //     //     packet_intern.dependencies(),
    //     // )?;
    //     self.add_hashed_packet(&packet.hash, packet_intern);

    //     let repository =
    //         definitions::find_repository_by_uuid_mut(&mut self.repositories, repository)
    //             .ok_or_else(|| {
    //                 Error::Repository(format!(
    //                     "Could not open repository \"{}\" for writing.",
    //                     repository
    //                 ))
    //             })?;
    //     if let Some(facet_name) = facet_name {
    //         repository.add_facet(facet_name);
    //     }

    //     repository.add_packet(&packet.data.name, &packet.hash);

    //     Ok(())
    // }

    fn fsck(&self) -> Result<bool> {
        self.repository_db.fsck()?;

        Ok(true)
    }
}
