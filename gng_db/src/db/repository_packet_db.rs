// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object associating a `Name` of a `Packet` with a `Hash` of the `Packet`.

use crate::{Result, Uuid};

use gng_shared::{Hash, Name};
use std::collections::BTreeMap;

// - Type aliases:
// ----------------------------------------------------------------------

type PacketsHashMap = BTreeMap<Name, Hash>;
type RepositoryPacketsMap = BTreeMap<Uuid, PacketsHashMap>;
type PacketUsageMap = BTreeMap<Hash, usize>;

// ----------------------------------------------------------------------
// - RepositoryPacketDb:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
#[derive(Clone, Debug)]
pub struct RepositoryPacketDb {
    repository_packet_db: RepositoryPacketsMap,
    packet_usage_db: PacketUsageMap,
}

impl RepositoryPacketDb {
    pub fn new() -> Result<Self> {
        todo!()
    }

    /// Resolve a `Packet` by its `name`, using a `search_path` of `Repository`s.
    pub fn resolve_packet(&self, name: &Name, search_path: &[&Uuid]) -> Option<(Hash, Uuid)> {
        search_path
            .iter()
            .map(|u| {
                (
                    self.repository_packet_db.get(*u).map(|pdb| pdb.get(name)),
                    *u,
                )
            })
            .find_map(|(h, u)| h.flatten().map(|hr| (hr.clone(), *u)))
    }

    /// Add a new `Packet` to the DB.
    pub fn add_packet(&mut self, repository: &Uuid, name: &Name, hash: &Hash) -> Result<()> {
        todo!()
    }

    /// Remove a `Packet` from the DB.
    /// Returns an `Option<Hash>` which will contain a Hash that is no longer used.
    pub fn remove_packet(&mut self, repository: &Uuid, name: &Name) -> Result<Option<Hash>> {
        todo!()
    }
}

impl Default for RepositoryPacketDb {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        Self {
            repository_packet_db: RepositoryPacketsMap::new(),
            packet_usage_db: PacketUsageMap::new(),
        }
    }
} // Default for DbImpl
