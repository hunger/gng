// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Definition of data types used by the `RepositoryDb`

use crate::{Error, Result};

use gng_shared::{Hash, Name, Packet};

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
