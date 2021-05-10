// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Definition of data types used by the `RepositoryDb`

use crate::{Error, Repository, Result};

use gng_shared::{Hash, Name, Names, Packet};

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn resolve_dependency(
    dependency_name: &Name,
    repository_group: &[&RepositoryIntern],
) -> Option<Hash> {
    for r in repository_group {
        if let Some(p) = r.packets().get(dependency_name).cloned() {
            return Some(p);
        }
    }
    None
}

fn resolve_dependencies(
    dependency_names: &Names,
    repository_group: &[&RepositoryIntern],
) -> Result<Vec<Hash>> {
    dependency_names
        .into_iter()
        .map(|n| {
            resolve_dependency(n, repository_group)
                .ok_or_else(|| Error::Packet(format!("Failed to resolve dependency \"{}\".", n)))
        })
        .collect()
}

// ----------------------------------------------------------------------
// - Types:
// ----------------------------------------------------------------------

pub type HashedPackets = std::collections::BTreeMap<Hash, PacketIntern>;
pub type RepositoryPackets = std::collections::BTreeMap<Name, Hash>;

// ----------------------------------------------------------------------
// - PacketIntern:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PacketIntern {
    data: Packet,
    facets: Vec<(Name, Hash)>,
    resolved_dependencies: Vec<Hash>,
    reverse_resolved_dependencies: Vec<Hash>,
}

impl PacketIntern {
    pub fn new(
        data: Packet,
        facets: Vec<(Name, Hash)>,
        repository_group: &[&RepositoryIntern],
    ) -> Result<Self> {
        for (facet_name, facet_hash) in &facets {
            if facet_hash.algorithm() == "none" {
                return Err(Error::Packet(format!(
                    "Facet \"{}\" has no hash.",
                    facet_name
                )));
            }

            // TODO: Check facet name based on repository group!
        }

        let resolved_dependencies = resolve_dependencies(&data.dependencies, repository_group)?;

        Ok(Self {
            data,
            facets,
            resolved_dependencies,
            reverse_resolved_dependencies: Vec::new(),
        })
    }

    pub const fn data(&self) -> &Packet {
        &self.data
    }

    pub fn dependencies(&self) -> &[Hash] {
        &self.resolved_dependencies
    }

    pub fn replace_reverse_resolved_dependency(
        &mut self,
        old: &Option<Hash>,
        new: &Option<Hash>,
    ) -> Result<()> {
        let rev = &mut self.reverse_resolved_dependencies;
        if let Some(old) = old {
            if let Some(idx) = rev.iter().position(|e| e == old) {
                rev.remove(idx);
            } else {
                return Err(Error::Packet(format!(
                    "Failed to remove reverse dependency from \"{}\".",
                    old
                )));
            }
        }
        if let Some(new) = new {
            rev.push(new.clone());
        }

        rev.sort();

        Ok(())
    }
}

// ----------------------------------------------------------------------
// - RepositoryIntern:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RepositoryIntern {
    repository: Repository,
    packets: RepositoryPackets,
    pub facets: Names,

    pub search_paths: Vec<crate::Uuid>,
    pub all_tags: Names,
}

impl RepositoryIntern {
    pub fn new(repository: Repository) -> Self {
        Self {
            repository,
            packets: RepositoryPackets::new(),
            facets: Names::default(),
            search_paths: Vec::new(),
            all_tags: Names::default(),
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

    pub fn add_packet(&mut self, name: &Name, hash: &Hash) {
        self.packets.insert(name.clone(), hash.clone());
    }

    pub fn all_tags(&self) -> &Names {
        &self.all_tags
    }

    #[must_use]
    pub fn is_local(&self) -> bool {
        self.repository().is_local()
    }

    #[must_use]
    pub fn is_override(&self) -> bool {
        self.repository().is_override()
    }
}

// - Helper for RepositoryIntern:
// ----------------------------------------------------------------------

pub fn find_repository_by_uuid_mut<'a, 'b>(
    repositories: &'a mut [RepositoryIntern],
    uuid: &'b crate::Uuid,
) -> Option<&'a mut RepositoryIntern> {
    repositories
        .iter_mut()
        .find(|r| r.repository().uuid == *uuid)
}

pub fn find_repository_by_uuid<'a, 'b>(
    repositories: &'a [RepositoryIntern],
    uuid: &'b crate::Uuid,
) -> Option<&'a RepositoryIntern> {
    repositories.iter().find(|r| r.repository().uuid == *uuid)
}

pub fn recursive_repository_dependencies<'a>(
    all_repositories: &'a [RepositoryIntern],
    base_repository: &'a RepositoryIntern,
) -> Vec<&'a RepositoryIntern> {
    let br = base_repository.repository();
    if let crate::RepositoryRelation::Dependency(dependencies) = &br.relation {
        let mut result = Vec::with_capacity(dependencies.len() + 1);
        result.push(base_repository);

        result.extend(dependencies.iter().flat_map(|u| {
            find_repository_by_uuid(all_repositories, u).map_or(Vec::new(), |r| {
                recursive_repository_dependencies(all_repositories, r)
            })
        }));
        result
    } else {
        Vec::new()
    }
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
