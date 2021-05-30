// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::Result;

// ----------------------------------------------------------------------
// - GngDbExt:
// ----------------------------------------------------------------------

/// Extension for `gng_shared::Gng` to add `gng_db` support.
pub trait GngDbExt {
    /// Open the `RepositoryDB` pointed to by this `Config`
    ///
    /// # Errors
    /// May return a `Error::Repository` if opening the Repository DB failed.
    fn repository_db(&self) -> Result<crate::repository_db::RepositoryDb>;

    /// Open the `RepositoryDB` pointed to by this `Config`
    ///
    /// # Errors
    /// May return a `Error::Repository` if opening the Repository DB failed.
    fn packet_db(&self) -> Result<crate::packet_db::PacketDb>;
}

impl GngDbExt for gng_shared::Gng {
    #[tracing::instrument(level = "debug")]
    fn repository_db(&self) -> Result<crate::repository_db::RepositoryDb> {
        crate::repository_db::RepositoryDb::open(&self.repository_dir)
    }

    #[tracing::instrument(level = "debug")]
    fn packet_db(&self) -> Result<crate::packet_db::PacketDb> {
        crate::packet_db::PacketDb::open(&self.packet_db_directory)
    }
}
