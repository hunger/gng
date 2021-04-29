// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

mod backend;
mod definitions;

// ----------------------------------------------------------------------
// - RepositoryDb:
// ----------------------------------------------------------------------

/// A `Repository` of gng `Packet`s and related information
pub trait RepositoryDb {
    /// Get the Schema version
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn schema_version(&self) -> crate::Result<u32>;

    // Repository management:

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn list_repositories(&self) -> crate::Result<Vec<crate::Repository>>;

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn add_repository(&mut self, repository_data: crate::Repository) -> crate::Result<()>;

    /// Remove a repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn remove_repository(&mut self, name: &gng_shared::Name) -> crate::Result<()>;

    // Debug things:

    /// Run sanity checks on Repository
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn fsck(&self) -> crate::Result<bool>;

    /// Print out the metadata stored about this repository.
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn dump_metadata(&mut self) -> crate::Result<()>;
}

// ----------------------------------------------------------------------
// - RepositoryDbImpl:
// ----------------------------------------------------------------------

pub(crate) struct RepositoryDbImpl {
    db: sled::Db,
    repositories: Vec<crate::Repository>,
}

impl RepositoryDbImpl {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn new(db: sled::Db) -> crate::Result<Self> {
        backend::open_db(&db)?;
        let repositories = backend::read_repositories(&db)?;

        Ok(Self { db, repositories })
    }
}

impl RepositoryDb for RepositoryDbImpl {
    fn schema_version(&self) -> crate::Result<u32> {
        backend::schema_version(&self.db)
    }

    fn list_repositories(&self) -> crate::Result<Vec<crate::Repository>> {
        Ok(self.repositories.clone())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn add_repository(&mut self, repository_data: crate::Repository) -> crate::Result<()> {
        // Are all dependencies known?
        let known_names: std::collections::HashSet<gng_shared::Name> =
            self.repositories.iter().map(|r| r.name.clone()).collect();
        let repository_name = repository_data.name.clone();

        repository_data
            .dependencies
            .into_iter()
            .find(|d| !known_names.contains(d))
            .map_or_else(
                || Ok(()),
                |n| {
                    Err(crate::Error::UnknownRepositoryDependency(
                        n.to_string(),
                        repository_name,
                    ))
                },
            )?;

        self.repositories.push(repository_data);
        self.repositories.sort();

        backend::write_repositories(&self.db, &self.repositories)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn remove_repository(&mut self, name: &gng_shared::Name) -> crate::Result<()> {
        let mut to_remove = self.repositories.len();
        let mut uuid = None;
        for (idx, r) in self.repositories.iter().enumerate() {
            if &r.name == name {
                to_remove = idx;
                uuid = Some(r.uuid);
            } else if r.dependencies.contains(name) {
                return Err(crate::Error::RepositoryInUse {
                    used_repository: name.clone(),
                    using_repository: r.name.clone(),
                });
            }
        }

        if let Some(uuid) = uuid {
            assert!(to_remove != self.repositories.len());
            self.repositories.remove(to_remove);

            backend::remove_repository(&self.db, &name.to_string(), &uuid)
        } else {
            Err(crate::Error::UnknownRepository(name.clone()))
        }
    }

    fn fsck(&self) -> crate::Result<bool> {
        Ok(true)
    }

    fn dump_metadata(&mut self) -> crate::Result<()> {
        backend::dump_metadata(&self.db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_repository() -> RepositoryDbImpl {
        let config = sled::Config::default().temporary(true);
        RepositoryDbImpl::new(config.open().expect("Temporary DB should have been valid!"))
            .expect("DB should have been created!")
    }

    #[test]
    fn test_repository_setup() {
        let _repo = create_repository();
    }
}
