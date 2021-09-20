// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A directory based packet DB

use gng_core::Name;

use eyre::{eyre, WrapErr};

// ----------------------------------------------------------------------
// - DirectoryDb:
// ----------------------------------------------------------------------

/// A packet DB that just gets packets from a given directory
#[derive(Debug, Clone)]
pub struct DirectoryDb {
    db_directory: String,
}

impl DirectoryDb {
    /// Constructor
    ///
    /// # Errors
    ///
    /// Errors out when the `db_directory` is not valid utf8.
    pub fn new(db_directory: &std::path::Path) -> eyre::Result<Self> {
        Ok(Self {
            db_directory: {
                let tmp = (db_directory.canonicalize())
                    .wrap_err("Could not canonicalize the DB directory.")?;
                let tmp = (tmp.to_str())
                    .ok_or_else(|| eyre!("Canonical DB directory is not utf8 encoded."))?;
                tmp.to_string()
            },
        })
    }
}

impl crate::Db for DirectoryDb {
    #[tracing::instrument(level = "trace")]
    fn knows(&self, packet: &Name, facet: &Option<Name>) -> bool {
        self.find(packet, facet).is_ok()
    }

    #[tracing::instrument(level = "trace")]
    fn find(&self, packet: &Name, facet: &Option<Name>) -> eyre::Result<std::path::PathBuf> {
        let full_packet_name = packet.combine(facet);
        let glob_result = glob::glob(&format!("{}/{}*.gng", self.db_directory, &full_packet_name,))
            .wrap_err("Failed to glob for packet.")?;
        (glob_result.last())
            .ok_or_else(|| {
                eyre!(
                    "Packet \"{}\" not found in \"{}\".",
                    &full_packet_name,
                    &self.db_directory
                )
            })?
            .wrap_err(eyre!(
                "Failed while globing for packet \"{}\".",
                &full_packet_name,
            ))
    }
}
