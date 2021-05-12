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
    replaces: Option<Hash>,
    replaced_by: Option<Hash>,
    facets: Vec<(Name, Hash)>,
    resolved_dependencies: Vec<Hash>,
    reverse_resolved_dependencies: Vec<Hash>,
}

impl PacketIntern {
    pub fn new(
        data: Packet,
        replaces: Option<Hash>,
        facets: Vec<(Name, Hash)>,
        resolved_dependencies: &[Hash],
    ) -> Result<Self> {
        for (facet_name, facet_hash) in &facets {
            if facet_hash.algorithm() == "none" {
                return Err(Error::Packet(format!(
                    "Facet \"{}\" has none hash.",
                    facet_name
                )));
            }
        }

        Ok(Self {
            data,
            facets,
            replaces,
            replaced_by: None,
            resolved_dependencies: resolved_dependencies.to_vec(),
            reverse_resolved_dependencies: Vec::new(),
        })
    }

    pub const fn data(&self) -> &Packet {
        &self.data
    }

    pub fn dependencies(&self) -> &[Hash] {
        &self.resolved_dependencies
    }

    pub fn replaces(&self) -> Option<Hash> {
        self.replaces.clone()
    }

    /// Replace a packet with a same one using a different Hash
    /// Returns the "breakage" that gets introduced into the system by this change.
    pub fn replace_by(&mut self, hash: &Hash) -> Result<Vec<Hash>> {
        if let Some(h) = &self.replaced_by {
            return Err(Error::Repository(format!(
                "Packet was already replaced by {}!",
                h
            )));
        }
        self.replaced_by = Some(hash.clone());
        Ok(self.reverse_resolved_dependencies.clone())
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
}

impl RepositoryIntern {
    pub fn new(repository: Repository) -> Self {
        Self {
            repository,
            packets: RepositoryPackets::new(),
            facets: Names::default(),
            search_paths: Vec::new(),
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

    #[must_use]
    pub const fn is_local(&self) -> bool {
        self.repository().is_local()
    }

    #[must_use]
    pub const fn is_override(&self) -> bool {
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
