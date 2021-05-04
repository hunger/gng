// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{Error, PacketData, Repository, Result, Uuid};

use self::definitions::{HashedPackets, PacketIntern, RepositoryIntern};
use gng_shared::{Hash, Name};

use std::convert::TryFrom;

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

#[must_use]
const fn is_local_repository(repository: &RepositoryIntern) -> bool {
    matches!(
        repository.repository().source,
        crate::RepositorySource::Local(_)
    )
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

fn validate_repositories_dependencies(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories {
        let r = r.repository();
        if let crate::RepositoryRelation::Dependency(dependencies) = &r.relation {
            for d in dependencies {
                if definitions::find_repository_by_uuid(repositories, d).is_none() {
                    return Err(Error::Repository(format!(
                        "Repository \"{}\" has unknown dependency \"{}\".",
                        &r.name, &d
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_repository_no_dependencies_cycles(
    all_repositories: &[RepositoryIntern],
    base: &RepositoryIntern,
    seen_uuids: &std::collections::HashSet<Uuid>,
) -> Result<()> {
    let mut seen_uuids = seen_uuids.clone();
    if seen_uuids.insert(base.repository().uuid) {
        if let crate::RepositoryRelation::Dependency(dependencies) = &base.repository().relation {
            for u in dependencies {
                let next_base = definitions::find_repository_by_uuid(all_repositories, u)
                    .ok_or_else(|| {
                        Error::Repository(format!("Unknown repository dependency \"{}\".", u))
                    })?;
                validate_repository_no_dependencies_cycles(
                    all_repositories,
                    next_base,
                    &seen_uuids,
                )?;
            }
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
            if r.repository().uuid != base_repository.repository().uuid {
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

struct RepositoryTreeNode<'a> {
    repository: &'a RepositoryIntern,

    overridden_by: Option<usize>,
    overrides: Option<usize>,
    depends_on: Vec<usize>,
    depended_on: Vec<usize>,
}

fn generate_repository_tree(repositories: &[RepositoryIntern]) -> Result<Vec<RepositoryTreeNode>> {
    let uuid_to_index: std::collections::HashMap<Uuid, usize> = repositories
        .iter()
        .enumerate()
        .map(|(idx, r)| (r.repository().uuid, idx))
        .collect();

    let mut tree_nodes = repositories
        .iter()
        .map(|ri| {
            let r = ri.repository();

            let overrides = if let crate::RepositoryRelation::Override(o) = &r.relation {
                Some(*uuid_to_index.get(o).ok_or_else(|| {
                    Error::Repository(format!(
                        "Repository \"{}\" ({}) overrides unknown repository \"{}\"",
                        &r.name, &r.uuid, o
                    ))
                })?)
            } else {
                None
            };

            let depends_on =
                if let crate::RepositoryRelation::Dependency(dependencies) = &r.relation {
                    dependencies
                        .iter()
                        .map(|u| {
                            uuid_to_index.get(u).copied().ok_or_else(|| {
                                Error::Repository(format!(
                                    "Repository \"{}\" ({}) depends on unknown repository \"{}\"",
                                    &r.name, &r.uuid, u
                                ))
                            })
                        })
                        .collect::<Result<Vec<_>>>()?
                } else {
                    Vec::new()
                };

            Ok(RepositoryTreeNode {
                repository: ri,
                overridden_by: None,
                overrides,
                depends_on,
                depended_on: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Second run: Fill in missing data
    for (idx, ri) in repositories.iter().enumerate() {
        let r = ri.repository();

        match &r.relation {
            crate::RepositoryRelation::Override(_) => {
                let o_idx = tree_nodes[idx]
                    .overrides
                    .expect("Must be valid for this kind of node!");

                let other = &mut tree_nodes[o_idx];
                if let Some(overridden_by) = other.overridden_by {
                    return Err(Error::Repository(format!(
                        "Repository \"{}\" ({}) is overridden by repositories \"{}\" ({}) and \"{}\" ({}).",
                        &other.repository.repository().name,
                        &other.repository.repository().uuid,
                        &r.name,
                        &r.uuid,
                        &tree_nodes[overridden_by].repository.repository().name,
                        &tree_nodes[overridden_by].repository.repository().uuid
                    )));
                }
                other.overridden_by = Some(idx);
            }
            crate::RepositoryRelation::Dependency(_) => {
                // Map depends_on to base nodes of overrides:
                let depends_on: Vec<_> = tree_nodes[idx]
                    .depends_on
                    .iter()
                    .map(|d_idx| {
                        let mut idx = *d_idx;
                        while let Some(override_idx) = tree_nodes[idx].overrides {
                            idx = override_idx;
                        }
                        idx
                    })
                    .collect();

                for d in &depends_on {
                    tree_nodes[*d].depended_on.push(idx);
                }

                tree_nodes[idx].depends_on = depends_on;
            }
        }
    }

    assert_eq!(tree_nodes.len(), repositories.len());

    Ok(tree_nodes)
}

fn deduplicate_uuids_in_search_path(input: Vec<Uuid>) -> Vec<Uuid> {
    assert!(!input.is_empty());

    let mut seen_uuids = std::collections::HashSet::new();

    let filtered: Vec<_> = input
        .into_iter()
        .rev()
        .filter(|u| seen_uuids.insert(*u))
        .collect();

    filtered.into_iter().rev().collect()
}

fn calculate_repository_search_path_for_node(
    nodes: &[RepositoryTreeNode],
    current_index: usize,
    mut result: Vec<Vec<Uuid>>,
) -> Vec<Vec<Uuid>> {
    assert!(result[current_index].is_empty());

    let mut node_result = Vec::new();

    // Handle overrides:
    let mut overridden_by = Some(current_index);
    while let Some(idx) = overridden_by {
        node_result.push(nodes[idx].repository.repository().uuid);
        overridden_by = nodes[idx].overridden_by;
    }
    node_result = node_result.into_iter().rev().collect(); // Last override comes first!

    // Handle Dependencies:
    for dependency_index in &nodes[current_index].depends_on {
        let dependency_index = *dependency_index;
        if result[dependency_index].is_empty() {
            result = calculate_repository_search_path_for_node(nodes, dependency_index, result);
        }
        node_result.extend_from_slice(&result[dependency_index][..]);
    }

    let node_result = deduplicate_uuids_in_search_path(node_result);

    // Fill in search path into overriding nodes
    let mut overridden_by = nodes[current_index].overridden_by;
    while let Some(idx) = overridden_by {
        result[idx] = node_result.clone();
        overridden_by = nodes[idx].overridden_by;
    }
    // ... and the main node!
    result[current_index] = node_result;

    result
}

fn calculate_repository_search_paths(
    repositories: &[RepositoryIntern],
) -> Result<(Vec<Vec<Uuid>>, Vec<Uuid>)> {
    let nodes = generate_repository_tree(repositories)?;

    let mut result = vec![Vec::new(); repositories.len()];
    let mut global_search_path = Vec::new();

    // Find leaf nodes:
    let leaf_indices: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter_map(|(idx, n)| {
            if n.depended_on.is_empty() && n.overrides.is_none() {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    for l in leaf_indices {
        result = calculate_repository_search_path_for_node(&nodes, l, result);
        global_search_path.extend_from_slice(&result[l][..]);
    }

    if result.iter().any(Vec::is_empty) {
        return Err(Error::Repository(
            "Failed to fill in search paths.".to_string(),
        ));
    }
    assert_eq!(result.len(), repositories.len());

    Ok((result, deduplicate_uuids_in_search_path(global_search_path)))
}

fn update_repository_search_paths(repositories: &mut [RepositoryIntern]) -> Result<Vec<Uuid>> {
    repositories.sort_by(|a, b| b.repository().priority.cmp(&a.repository().priority));

    let (search_paths, global_repository_search_path) =
        calculate_repository_search_paths(repositories)?;

    for (idx, r) in repositories.iter_mut().enumerate() {
        r.search_paths = search_paths[idx].clone();
    }

    Ok(global_repository_search_path)
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
    fn adopt_packet(&mut self, packet: PacketData, repository: &Uuid) -> Result<()>;

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
    fn dump_metadata(&mut self);
}

// ----------------------------------------------------------------------
// - RepositoryDbImpl:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct RepositoryDbImpl {
    db_directory: Option<std::path::PathBuf>,

    repositories: Vec<RepositoryIntern>,
    global_repository_search_path: Vec<Uuid>,
    hashed_packets: HashedPackets,
}

impl RepositoryDbImpl {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn new(db_directory: &std::path::Path) -> Result<Self> {
        let (mut repositories, hashed_packets) = backend::read_db(db_directory)?;
        validate_repositories(&repositories)?;
        let global_repository_search_path = update_repository_search_paths(&mut repositories)?;

        Ok(Self {
            db_directory: Some(db_directory.to_owned()),
            repositories,
            global_repository_search_path,
            hashed_packets,
        })
    }

    // Return a tuple with the PacketIntern and an Optional facet name to register!
    #[tracing::instrument(level = "trace")]
    fn validate_packet_to_adopt(
        &mut self,
        packet: &PacketData,
        repository: &Uuid,
    ) -> Result<(PacketIntern, Option<Name>, Option<Hash>)> {
        // TODO: Check for cyclic dependencies in packets or facets

        let packet_name = &packet.data.name;
        let repository = definitions::find_repository_by_uuid(&self.repositories, repository)
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
            packet.data.clone(),
            packet.facets.clone(),
            &dependency_group,
        )?;

        let old_hash = repository.packets().get(packet_name).cloned();

        Ok((packet_intern, facet_name, old_hash))
    }

    fn add_hashed_packet(&mut self, hash: &Hash, data: PacketIntern) {
        self.hashed_packets.insert(hash.clone(), data);
    }

    fn fix_reverse_dependencies(
        &mut self,
        old_hash: &Option<Hash>,
        new_hash: &Option<Hash>,
        resolved_dependencies: &[Hash],
    ) -> Result<()> {
        for d in resolved_dependencies {
            if let Some(p) = self.hashed_packets.get_mut(d) {
                p.replace_reverse_resolved_dependency(old_hash, new_hash)?;
            } else {
                return Err(Error::Packet(format!(
                    "Failed to find dependency \"{}\".",
                    d
                )));
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
            global_repository_search_path: Vec::new(),
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

    fn resolve_repository(&self, input: &str) -> Option<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(input) {
            definitions::find_repository_by_uuid(&self.repositories, &uuid).map(|_| uuid)
        } else if let Ok(name) = Name::try_from(input) {
            self.repositories
                .iter()
                .find(|r| r.repository().name == name)
                .map(|ri| ri.repository().uuid)
        } else {
            None
        }
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
        validate_repositories(&repositories)?;
        let global_repository_search_path = update_repository_search_paths(&mut repositories)?;

        self.repositories = repositories;
        self.global_repository_search_path = global_repository_search_path;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn remove_repository(&mut self, uuid: &Uuid) -> Result<()> {
        let mut repositories: Vec<_> = self
            .repositories
            .iter()
            .filter(|r| r.repository().uuid != *uuid)
            .cloned()
            .collect();
        if repositories.len() == self.repositories.len() {
            return Err(Error::Repository(format!(
                "Repository \"{}\" not found, can not remove.",
                uuid
            )));
        }

        validate_repositories(&repositories)?;
        let global_repository_search_path = update_repository_search_paths(&mut repositories)?;

        self.repositories = repositories;
        self.global_repository_search_path = global_repository_search_path;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn adopt_packet(&mut self, packet: PacketData, repository: &Uuid) -> Result<()> {
        let (packet_intern, facet_name, old_packet_hash) =
            self.validate_packet_to_adopt(&packet, repository)?;

        self.fix_reverse_dependencies(
            &old_packet_hash,
            &Some(packet.hash.clone()),
            packet_intern.dependencies(),
        )?;
        self.add_hashed_packet(&packet.hash, packet_intern);

        let repository =
            definitions::find_repository_by_uuid_mut(&mut self.repositories, repository)
                .ok_or_else(|| {
                    Error::Repository(format!(
                        "Could not open repository \"{}\" for writing.",
                        repository
                    ))
                })?;
        if let Some(facet_name) = facet_name {
            repository.add_facet(facet_name);
        }

        repository.add_packet(&packet.data.name, &packet.hash);

        Ok(())
    }

    fn fsck(&self) -> Result<bool> {
        validate_repositories(&self.repositories)?;
        calculate_repository_search_paths(&self.repositories)?;

        Ok(true)
    }

    fn dump_metadata(&mut self) {
        println!("Repositories:");
        for r in &self.repositories {
            let locality_str = if is_local_repository(r) {
                "LOCAL"
            } else {
                "REMOTE"
            };
            let r = r.repository();
            let tags_str = if r.tags.is_empty() {
                "[]".to_string()
            } else {
                format!("[{}]", r.tags)
            };
            println!("{}: {} - {} ({})", r.priority, r.name, r.uuid, locality_str);
            println!("    Tags: {}", tags_str)
        }
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
            uuid: Uuid::new_v4(),
            priority: 100,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![]),
            tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("ext_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 1500,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![db
                .resolve_repository("base_repo")
                .expect("Repo was valid!")]),
            tags: Names::default(),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("tagged_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 1200,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/tagged"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![]),
            tags: Names::try_from(vec!["test1", "other_tag"]).expect("Names were valid!"),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("unrelated_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 6000,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/unrelated"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![]),
            tags: Names::default(),
        })
        .unwrap();
    }

    #[test]
    fn test_repository_validation_ok() {
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
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
                uuid: Uuid::new_v4(),
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::default(),
            }),
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_duplicate_uuid() {
        let uuid = Uuid::new_v4();
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::default(),
            }),
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_dependency_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: uuid2,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: uuid1,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
                tags: Names::default(),
            }),
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_unknown_dependency() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [RepositoryIntern::new(Repository {
            name: Name::try_from("base_repo").expect("Name was valid!"),
            uuid: uuid1,
            priority: 100,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![uuid2]),
            tags: Names::from(Name::try_from("test1").expect("Name was valid!")),
        })];

        assert!(validate_repositories(&repositories).is_err());
    }

    fn create_dependent_repo(name: &str, uuid: &Uuid, dependencies: Vec<Uuid>) -> RepositoryIntern {
        RepositoryIntern::new(Repository {
            name: Name::try_from(name).expect("Name was valid!"),
            uuid: *uuid,
            priority: 1500,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from(format!(
                    "file:///tmp/sources/{}",
                    &name
                )),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(dependencies),
            tags: Names::default(),
        })
    }

    fn create_override_repo(name: &str, uuid: &Uuid, overrides: Uuid) -> RepositoryIntern {
        RepositoryIntern::new(Repository {
            name: Name::try_from(name).expect("Name was valid!"),
            uuid: *uuid,
            priority: 1500,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from(format!(
                    "file:///tmp/sources/{}",
                    &name
                )),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Override(overrides),
            tags: Names::default(),
        })
    }

    #[test]
    fn test_update_search_paths_line() {
        let uuid_0 = Uuid::new_v4();
        let uuid_1 = Uuid::new_v4();
        let uuid_1o0 = Uuid::new_v4();
        let uuid_2 = Uuid::new_v4();
        let uuid_2o0 = Uuid::new_v4();
        let uuid_2o1 = Uuid::new_v4();
        let uuid_3 = Uuid::new_v4();

        let mut repositories = [
            create_dependent_repo("r3", &uuid_3, vec![uuid_2]),
            create_override_repo("r1o0", &uuid_1o0, uuid_1),
            create_override_repo("r2o1", &uuid_2o1, uuid_2o0),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0]),
            create_override_repo("r2o0", &uuid_2o0, uuid_2),
            create_dependent_repo("r0", &uuid_0, vec![]),
            create_dependent_repo("r2", &uuid_2, vec![uuid_1]),
        ];

        for r in &repositories {
            println!("{}", r.repository().to_pretty_string());
        }

        let global_search_path = update_repository_search_paths(&mut repositories)
            .expect("Input was supposed to be correct");

        assert_eq!(
            global_search_path,
            vec![uuid_3, uuid_2o1, uuid_2o0, uuid_2, uuid_1o0, uuid_1, uuid_0]
        )
    }

    #[test]
    fn test_update_search_paths_diamond() {
        let uuid_0 = Uuid::new_v4();
        let uuid_1 = Uuid::new_v4();
        let uuid_2l0 = Uuid::new_v4();
        let uuid_2l1 = Uuid::new_v4();
        let uuid_2r0 = Uuid::new_v4();
        let uuid_2r0o0 = Uuid::new_v4();
        let uuid_3 = Uuid::new_v4();

        let mut repositories = [
            create_dependent_repo("r0", &uuid_0, vec![]),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0]),
            create_dependent_repo("r2l0", &uuid_2l0, vec![uuid_1]),
            create_dependent_repo("r2l1", &uuid_2l1, vec![uuid_2l0]),
            create_dependent_repo("r2r0", &uuid_2r0, vec![uuid_1]),
            create_override_repo("r2r0o0", &uuid_2r0o0, uuid_2r0),
            create_dependent_repo("r3", &uuid_3, vec![uuid_2l1, uuid_2r0]),
        ];

        for r in &repositories {
            println!("{}", r.repository().to_pretty_string());
        }

        let global_search_path = update_repository_search_paths(&mut repositories)
            .expect("Input was supposed to be correct");

        assert_eq!(
            global_search_path,
            vec![uuid_3, uuid_2l1, uuid_2l0, uuid_2r0o0, uuid_2r0, uuid_1, uuid_0,]
        )
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
                &repo_db
                    .resolve_repository("ext_repo")
                    .expect("Repo was valid"),
            )
            .unwrap();

        repo_db.dump_metadata();
        panic!("Always fail for now!");
    }
}
