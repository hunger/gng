// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A trait extending `gng_shared::GnG` for `gng_db`.

// ----------------------------------------------------------------------
// - GngDbExt:
// ----------------------------------------------------------------------

/// Extension for `gng_shared::Gng` to add `gng_db` support.
pub trait GngDbExt {
    // /// Open the `RepositoryDB` pointed to by this `Config`
    // ///
    // /// # Errors
    // /// May return a `Error::Repository` if opening the Repository DB failed.
    // fn db(&self) -> Result<crate::db::Db>;
}

impl GngDbExt for gng_shared::Gng {
    // #[tracing::instrument(level = "debug")]
    // fn db(&self) -> Result<crate::db::Db> {
    //     let file = self.db_directory.join("gng.db3");
    //     crate::db::Db::open(Some(&file))
    // }
}
