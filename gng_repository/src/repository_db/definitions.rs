// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Definition of data types used by the `RepositoryDb`

use crate::{Error, Repository, Result};

use gng_shared::{Hash, Name, Names, Packet};

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn resolve_dependency(
    packet_hash: &Hash,
    dependency_name: &Name,
    repository_group: &[&RepositoryIntern],
) -> Option<Hash> {
    todo!()
}

fn resolve_dependencies(
    packet_hash: &Hash,
    dependency_names: &Names,
    repository_group: &[&RepositoryIntern],
) -> Result<Vec<Hash>> {
    dependency_names
        .into_iter()
        .map(|n| {
            resolve_dependency(packet_hash, n, repository_group)
                .ok_or_else(|| Error::Packet(format!("Failed to resolve dependency \"{}\".", n)))
        })
        .collect()
}

fn check_dependencies(dependencies: &[Hash], base_repository: &RepositoryIntern) -> Result<()> {
    todo!()
}

// ----------------------------------------------------------------------
// - Types:
// ----------------------------------------------------------------------

pub type HashedPackets = std::collections::BTreeMap<Hash, Packet>;
pub type RepositoryPackets = std::collections::BTreeMap<Name, PacketIntern>;

// ----------------------------------------------------------------------
// - PacketIntern:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, Eq, serde::Deserialize, serde::Serialize)]
pub struct PacketIntern {
    hash: Hash,
    facets: Vec<(Name, Hash)>,
    resolved_dependencies: Vec<Hash>,
    reverse_resolved_dependencies: Vec<Hash>,
}

impl PacketIntern {
    pub fn new(
        hash: Hash,
        data: &Packet,
        facets: Vec<(Name, Hash)>,
        repository_group: &[&RepositoryIntern],
    ) -> Result<Self> {
        let base_repository = if let Some(br) = repository_group.first() {
            *br
        } else {
            return Err(Error::Packet(
                "Repository dependencies are broken.".to_string(),
            ));
        };

        if hash.algorithm() == "none" {
            return Err(Error::Packet("No packet hash was provided.".to_string()));
        }

        for (facet_name, facet_hash) in &facets {
            if facet_hash.algorithm() == "none" {
                return Err(Error::Packet(format!(
                    "Facet \"{}\" has no hash.",
                    facet_name
                )));
            }

            // TODO: Check facet name based on repository group!
        }

        let resolved_dependencies =
            resolve_dependencies(&hash, &data.dependencies, repository_group)?;

        check_dependencies(&resolved_dependencies, base_repository)?;

        Ok(Self {
            hash,
            facets,
            resolved_dependencies,
            reverse_resolved_dependencies: Vec::new(),
        })
    }

    pub fn hash(&self) -> Hash {
        self.hash.clone()
    }
}

impl std::cmp::Ord for PacketIntern {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl std::cmp::PartialEq for PacketIntern {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl std::cmp::PartialOrd for PacketIntern {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ----------------------------------------------------------------------
// - RepositoryIntern:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, Eq, serde::Deserialize, serde::Serialize)]
pub struct RepositoryIntern {
    repository: Repository,
    packets: RepositoryPackets,
    facets: Names,
}

impl RepositoryIntern {
    pub fn new(repository: Repository) -> Self {
        Self {
            repository,
            packets: RepositoryPackets::new(),
            facets: Names::default(),
        }
    }

    pub const fn repository(&self) -> &Repository {
        &self.repository
    }

    pub const fn packets(&self) -> &RepositoryPackets {
        &self.packets
    }

    pub const fn facets(&self) -> &Names {
        &self.facets
    }

    pub fn add_facet(&mut self, name: Name) {
        self.facets.insert(name);
    }

    pub fn add_packet(&mut self, name: &Name, packet: PacketIntern) {
        self.packets.insert(name.clone(), packet);
    }
}

impl std::cmp::Ord for RepositoryIntern {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.repository.cmp(&other.repository)
    }
}

impl std::cmp::PartialEq for RepositoryIntern {
    fn eq(&self, other: &Self) -> bool {
        self.repository == other.repository
    }
}

impl std::cmp::PartialOrd for RepositoryIntern {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// - Helper for RepositoryIntern:
// ----------------------------------------------------------------------

pub fn find_repository_by_name_mut<'a>(
    repositories: &'a mut [RepositoryIntern],
    name: &'a Name,
) -> Option<&'a mut RepositoryIntern> {
    repositories
        .iter_mut()
        .find(|r| r.repository().name == *name)
}

pub fn find_repository_by_name<'a>(
    repositories: &'a [RepositoryIntern],
    name: &'a Name,
) -> Option<&'a RepositoryIntern> {
    repositories.iter().find(|r| r.repository().name == *name)
}

pub fn recursive_repository_dependencies<'a>(
    all_repositories: &'a [RepositoryIntern],
    base_repository: &'a RepositoryIntern,
) -> Vec<&'a RepositoryIntern> {
    let br = base_repository.repository();
    let mut result = Vec::with_capacity(br.dependencies.len() + 1);
    result.push(base_repository);

    result.extend(br.dependencies.into_iter().flat_map(|n| {
        find_repository_by_name(all_repositories, n).map_or(Vec::new(), |r| {
            recursive_repository_dependencies(all_repositories, r)
        })
    }));

    result
}

// Return all *other* repositories that match at least one tag of the base_repository.
// The base_repository will not be in the result set!
pub fn repository_tags_group<'a>(
    all_repositories: &'a [RepositoryIntern],
    base_repository: &'a RepositoryIntern,
) -> Vec<&'a RepositoryIntern> {
    let br = base_repository.repository();
    let base_tags: std::collections::HashSet<Name> = br.tags.into_iter().cloned().collect();
    let base_uuid = base_repository.repository().uuid;

    all_repositories
        .iter()
        .filter(|r| {
            r.repository().uuid != base_uuid
                && r.repository()
                    .tags
                    .into_iter()
                    .any(|t| base_tags.contains(t))
        })
        .collect()
}

pub fn find_facet_implementation_repository<'a, 'b>(
    dependent_repositories: &'a [&'a RepositoryIntern],
    facet_name: &'b Name,
) -> Option<&'a RepositoryIntern> {
    dependent_repositories.iter().find_map(|r| {
        if r.facets().contains(facet_name) {
            Some(*r)
        } else {
            None
        }
    })
}
