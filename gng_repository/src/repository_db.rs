// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{Error, PacketData, Repository, Result};

use self::definitions::{HashedPackets, PacketIntern, RepositoryIntern};
use gng_shared::{Hash, Name};

mod backend;
mod definitions;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn validate_repositories_uniqueness(repositories: &[definitions::RepositoryIntern]) -> Result<()> {
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

fn validate_repositories_priority(repositories: &[RepositoryIntern]) -> Result<()> {
    let mut current_priority = u32::MAX;

    for r in repositories {
        let r = r.repository();
        if r.priority > current_priority {
            return Err(Error::Repository(
                "Repositories are not ordered based on their priority.".to_string(),
            ));
        }
        current_priority = r.priority;
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

fn validate_local_repository(repository: &RepositoryIntern) -> Result<()> {
    let r = repository.repository();
    if r.pull_url.is_some() {
        Err(Error::Repository(format!(
            "The local repository \"{}\" may not have a pull_url defined.",
            &r.name
        )))
    } else if !validate_url(&r.packet_base_url)? {
        Err(Error::Repository(format!(
            "The local repository \"{}\" must have a file:-url as packet_base_url.",
            &r.name
        )))
    } else if r.sources_base_directory.is_none() {
        Err(Error::Repository(format!(
            "The local repository \"{}\" must have a sources_base_directory set.",
            &r.name
        )))
    } else {
        Ok(())
    }
}

fn validate_remote_repository(repository: &RepositoryIntern) -> Result<()> {
    let r = repository.repository();
    if r.pull_url.is_none() {
        Err(Error::Repository(format!(
            "The remote repository \"{}\" must have a pull_url defined.",
            &r.name
        )))
    } else if validate_url(&r.packet_base_url)? {
        Err(Error::Repository(format!(
            "The remote repository \"{}\" must have a http(s):-url as packet_base_url.",
            &r.name
        )))
    } else if r.sources_base_directory.is_some() {
        Err(Error::Repository(format!(
            "The remote repository \"{}\" may not have a sources_base_directory set.",
            &r.name
        )))
    } else {
        Ok(())
    }
}

#[must_use]
const fn is_local_repository(repository: &RepositoryIntern) -> bool {
    repository.repository().pull_url.is_none()
}

fn validate_repositories_urls_and_sources(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories {
        if is_local_repository(r) {
            validate_local_repository(r)?;
        } else {
            validate_remote_repository(r)?;
        }
    }

    Ok(())
}

fn validate_repositories_dependencies(repositories: &[RepositoryIntern]) -> Result<()> {
    let known_names: std::collections::HashSet<Name> = repositories
        .iter()
        .map(|r| r.repository().name.clone())
        .collect();

    for r in repositories {
        for d in &r.repository().dependencies {
            if !known_names.contains(d) {
                return Err(Error::Repository(format!(
                    "Repository \"{}\" has unknown dependency \"{}\".",
                    &r.repository().name,
                    d
                )));
            }
        }
    }

    Ok(())
}

fn validate_repository_no_dependencies_cycles(
    all_repositories: &[RepositoryIntern],
    base: &RepositoryIntern,
    seen_uuids: &std::collections::HashSet<crate::Uuid>,
) -> Result<()> {
    let mut seen_uuids = seen_uuids.clone();
    if seen_uuids.insert(base.repository().uuid) {
        for d in &base.repository().dependencies {
            let next_base =
                definitions::find_repository_by_name(all_repositories, d).ok_or_else(|| {
                    Error::Repository(format!("Unknown repository dependency \"{}\".", d))
                })?;
            validate_repository_no_dependencies_cycles(all_repositories, next_base, &seen_uuids)?;
        }
        Ok(())
    } else {
        Err(Error::Repository(
            "Dependency cycle in Repository list detected!".to_string(),
        ))
    }
}

fn validate_repositories_no_dependencies_cycles(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories.iter() {
        let seen_uuids = std::collections::HashSet::new();
        validate_repository_no_dependencies_cycles(repositories, r, &seen_uuids)?;
    }
    Ok(())
}

fn validate_repositories(repositories: &[RepositoryIntern]) -> Result<()> {
    validate_repositories_uniqueness(repositories)?;
    validate_repositories_priority(repositories)?;
    validate_repositories_urls_and_sources(repositories)?;
    validate_repositories_dependencies(repositories)?;
    validate_repositories_no_dependencies_cycles(repositories)?;

    Ok(())
}

fn repository_contains_packet<'a, 'b>(
    repository: &'a RepositoryIntern,
    packet_name: &'b Name,
) -> Option<&'a RepositoryIntern> {
    repository
        .packets()
        .contains_key(packet_name)
        .then(|| repository)
}

fn repository_group_contains_packet<'a, 'b>(
    group: &'a [&'a RepositoryIntern],
    packet_name: &'b Name,
) -> Option<&'a RepositoryIntern> {
    group
        .iter()
        .map(|r| repository_contains_packet(r, packet_name))
        .find(std::option::Option::is_some)
        .unwrap_or_default()
}

fn valid_facet_name(
    packet: &PacketData,
    base_repository: &RepositoryIntern,
    group: &[&RepositoryIntern],
) -> Result<Option<Name>> {
    if packet.data.facet.is_some() {
        let facet_name = &packet.data.name;
        if let Some(r) = definitions::find_facet_implementation_repository(group, facet_name) {
            if r != base_repository {
                return Err(Error::Packet(format!(
                    "Facet \"{}\" is already implemented in repository \"{}\".",
                    facet_name,
                    &r.repository().name
                )));
            }
        }
        return Ok(Some(facet_name.clone()));
    }
    Ok(None)
}

// ----------------------------------------------------------------------
// - RepositoryDb:
// ----------------------------------------------------------------------

/// A `Repository` of gng `Packet`s and related information
pub trait RepositoryDb {
    /// Persist all data to storage
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn persist(&self) -> Result<()>;

    // Repository management:

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
    fn remove_repository(&mut self, name: &Name) -> Result<()>;

    // Packet management:

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn adopt_packet(&mut self, packet: PacketData, repository: &Name) -> Result<()>;

    // Debug things:

    /// Run sanity checks on Repository
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn fsck(&self) -> Result<bool>;

    /// Print out the metadata stored about this repository.
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn dump_metadata(&mut self) -> Result<()>;
}

// ----------------------------------------------------------------------
// - RepositoryDbImpl:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct RepositoryDbImpl {
    db_directory: Option<std::path::PathBuf>,

    repositories: Vec<RepositoryIntern>,
    hashed_packets: HashedPackets,
}

impl RepositoryDbImpl {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn new(db_directory: &std::path::Path) -> Result<Self> {
        let (repositories, hashed_packets) = backend::read_db(db_directory)?;
        validate_repositories(&repositories)?;

        Ok(Self {
            db_directory: Some(db_directory.to_owned()),
            repositories,
            hashed_packets,
        })
    }

    // Return a tuple with the PacketIntern and an Optional facet name to register!
    fn validate_packet_to_adopt(
        &mut self,
        packet: &PacketData,
        repository: &Name,
    ) -> Result<(PacketIntern, Option<Name>)> {
        // TODO: Check for cyclic dependencies in packets or facets

        let packet_name = &packet.data.name;
        let repository = definitions::find_repository_by_name(&self.repositories, repository)
            .ok_or_else(|| {
                Error::Repository(format!("Repository \"{}\" not found.", repository))
            })?;

        // Are we adopting into a local repository?
        if !is_local_repository(repository) {
            return Err(Error::Packet(
                "Can not adopt a Packet into a remote repository.".to_string(),
            ));
        }

        // Check for duplicate packet names in tag group:
        let tag_group = definitions::repository_tags_group(&self.repositories, repository);
        if let Some(ri) = repository_group_contains_packet(&tag_group, packet_name) {
            return Err(Error::Packet(format!(
                "Packet \"{}\" is already defined in repository \"{}\".",
                packet_name,
                &ri.repository().name
            )));
        }

        let dependency_group =
            definitions::recursive_repository_dependencies(&self.repositories, repository);
        let facet_name = valid_facet_name(packet, repository, &dependency_group)?;

        let packet_intern = PacketIntern::new(
            packet.hash.clone(),
            &packet.data,
            packet.facets.clone(),
            &dependency_group,
        )?;

        Ok((packet_intern, facet_name))
    }

    pub fn add_hashed_packet(&mut self, hash: &Hash, data: &gng_shared::Packet) -> Result<()> {
        let old_data = self.hashed_packets.insert(hash.clone(), data.clone());
        if let Some(old_data) = old_data {
            if old_data != *data {
                return Err(Error::Packet(format!("Replaced packet \"{}\" with different data, even though they have the same hash.", &old_data.name)));
            }
        }
        Ok(())
    }
}

impl Default for RepositoryDbImpl {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        Self {
            db_directory: None,
            repositories: Vec::new(),
            hashed_packets: HashedPackets::new(),
        }
    }
} // Default for RepositoryDbImpl

impl RepositoryDb for RepositoryDbImpl {
    #[tracing::instrument(level = "trace")]
    fn persist(&self) -> Result<()> {
        if let Some(db_directory) = &self.db_directory {
            backend::persist_repositories(db_directory, &self.repositories)?;
            tracing::debug!(
                "Repository DB was persisted to \"{}\".",
                db_directory.to_string_lossy()
            );
        }

        Ok(())
    }

    fn list_repositories(&self) -> Vec<Repository> {
        self.repositories
            .iter()
            .map(|r| r.repository().clone())
            .collect()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn add_repository(&mut self, repository_data: Repository) -> Result<()> {
        let mut repositories = self.repositories.clone();

        repositories.push(definitions::RepositoryIntern::new(repository_data));
        repositories.sort();
        validate_repositories(&repositories)?;

        self.repositories = repositories;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn remove_repository(&mut self, name: &Name) -> Result<()> {
        let repositories: Vec<_> = self
            .repositories
            .iter()
            .filter(|r| r.repository().name != *name)
            .cloned()
            .collect();
        if repositories.len() == self.repositories.len() {
            return Err(Error::Repository(format!(
                "Repository \"{}\" not found, can not remove.",
                name
            )));
        }

        validate_repositories(&repositories)?;

        self.repositories = repositories;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn adopt_packet(&mut self, packet: PacketData, repository: &Name) -> Result<()> {
        let (packet_intern, facet_name) = self.validate_packet_to_adopt(&packet, repository)?;

        self.add_hashed_packet(&packet.hash, &packet.data)?;

        let repository =
            definitions::find_repository_by_name_mut(&mut self.repositories, repository)
                .ok_or_else(|| {
                    Error::Repository(format!(
                        "Could not open repository \"{}\" for writing.",
                        repository
                    ))
                })?;
        if let Some(facet_name) = facet_name {
            repository.add_facet(facet_name);
        }

        repository.add_packet(&packet.data.name, packet_intern);

        Ok(())
    }

    fn fsck(&self) -> Result<bool> {
        validate_repositories(&self.repositories)?;

        Ok(true)
    }

    fn dump_metadata(&mut self) -> Result<()> {
        // backend::dump_metadata(&self.db)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::{From, TryFrom};

    use gng_shared::{Names, Packet, Version};

    use crate::Repository;

    use super::*;

    fn populate_repository_db(db: &mut RepositoryDbImpl) {
        db.add_repository(Repository {
            name: Name::try_from("base_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 100,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/base"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: Names::default(),
            tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("ext_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 1500,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/ext"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: Names::from(Name::try_from("base_repo").expect("Name was valid!")),
            tags: Names::default(),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("tagged_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 1200,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/tagged"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: Names::default(),
            tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("unrelated_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 6000,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/unrelated"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: Names::default(),
            tags: Names::default(),
        })
        .unwrap();
    }

    #[test]
    fn test_repository_validation_ok() {
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::default(),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::from(Name::try_from("base_repo").expect("Name was valid!")),
                tags: Names::default(),
            }),
        ];

        validate_repositories(&repositories).unwrap();
    }

    #[test]
    fn test_repository_validation_duplicate_name() {
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::default(),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::from(Name::try_from("base_repo").expect("Name was valid!")),
                tags: Names::default(),
            }),
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_duplicate_uuid() {
        let uuid = crate::Uuid::new_v4();
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid,
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::default(),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid,
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::from(Name::try_from("base_repo").expect("Name was valid!")),
                tags: Names::default(),
            }),
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_dependency_loop() {
        let uuid = crate::Uuid::new_v4();
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid,
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::from(Name::try_from("ext_repo").expect("Name was valid!")),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid,
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: Names::from(Name::try_from("base_repo").expect("Name was valid!")),
                tags: Names::default(),
            }),
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_unknown_dependency() {
        let uuid = crate::Uuid::new_v4();
        let repositories = [RepositoryIntern::new(Repository {
            name: Name::try_from("ext_repo").expect("Name was valid!"),
            uuid,
            priority: 1500,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/ext"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: Names::from(Name::try_from("base_repo").expect("Name was valid!")),
            tags: Names::default(),
        })];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_setup() {
        let mut repo_db = RepositoryDbImpl::default();
        populate_repository_db(&mut repo_db);

        let repositories = repo_db.list_repositories();

        let mut it = repositories.iter();

        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("unrelated_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("ext_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("tagged_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("base_repo")
        );
        assert!(it.next().is_none());
    }

    #[test]
    fn test_adopt_packet() {
        let mut repo_db = RepositoryDbImpl::default();
        populate_repository_db(&mut repo_db);

        let hash = Hash::try_from(
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .expect("Sha was valid!");

        repo_db
            .adopt_packet(
                PacketData {
                    facets: Vec::new(),
                    data: Packet {
                        source_name: Name::try_from("foobar").expect("Name was valid!"),
                        version: Version::try_from("1.0").expect("Version was valid!"),
                        license: "FooBar License!".to_string(),
                        name: Name::try_from("baz").expect("Name was valid"),
                        description: "Some description of baz packet".to_string(),
                        url: None,
                        bug_url: None,
                        dependencies: Names::default(),
                        facet: None,
                    },
                    hash,
                },
                &Name::try_from("ext_repo").expect("Name was valid!"),
            )
            .unwrap();
    }
}
