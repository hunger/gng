// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Db` with all the data on `Packet`s and related data.

mod backend;
mod repository_db;
mod repository_packet_db;

use crate::{Repository, Result, Uuid};

use self::{repository_db::RepositoryDb, repository_packet_db::RepositoryPacketDb};

// ----------------------------------------------------------------------
// - Db:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
pub trait Db {
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
    // fn adopt_packet(&mut self, packet: PacketData, repository: &Uuid) -> Result<()>;

    // Debug things:

    /// Run sanity checks on Repository
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn fsck(&self) -> Result<bool>;
}

// ----------------------------------------------------------------------
// - DbImpl:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct DbImpl {
    db_directory: Option<std::path::PathBuf>,

    repository_db: RepositoryDb,
    repository_packet_db: RepositoryPacketDb,
}

impl DbImpl {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn load(&mut self, db_directory: &std::path::Path) -> Result<()> {
        self.db_directory = None;

        self::backend::init_db(db_directory)?;

        // FIXME: Implement loading from disk

        self.db_directory = Some(db_directory.to_owned());
        todo!()
    }
}

impl Default for DbImpl {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        Self {
            db_directory: None,
            repository_db: RepositoryDb::default(),
            repository_packet_db: RepositoryPacketDb::default(),
        }
    }
} // Default for DbImpl

impl Db for DbImpl {
    fn resolve_repository(&self, input: &str) -> Option<Uuid> {
        self.repository_db.resolve_repository(input)
    }

    fn list_repositories(&self) -> Vec<Repository> {
        self.repository_db.all_repositories()
    }

    fn add_repository(&mut self, repository_data: Repository) -> Result<()> {
        if let Some(db_directory) = &self.db_directory {
            backend::persist_repository(db_directory, &repository_data)?;
        }

        self.repository_db.add_repository(repository_data)
    }

    fn remove_repository(&mut self, uuid: &Uuid) -> Result<()> {
        let repository_name = self.repository_db.repository(uuid)?.name;
        self.repository_db.remove_repository(uuid)?;
        self.db_directory.as_ref().map_or(Ok(()), |dd| {
            backend::remove_repository(dd, &repository_name)
        })
    }

    fn fsck(&self) -> Result<bool> {
        self.repository_db.fsck()?;

        Ok(true)
    }
}
