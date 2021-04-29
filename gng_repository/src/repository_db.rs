// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::Repository;

mod backend;
mod definitions;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn validate_repositories_uniqueness(repositories: &[crate::Repository]) -> crate::Result<()> {
    let mut known_names = std::collections::HashSet::new();
    let mut known_uuids = std::collections::HashSet::new();

    for r in repositories {
        if !known_names.insert(r.name.clone()) {
            return Err(crate::Error::Repository(format!(
                "Repository name \"{}\" is not unique.",
                &r.name
            )));
        }
        if !known_uuids.insert(r.uuid) {
            return Err(crate::Error::Repository(format!(
                "Repository UUID \"{}\" is not unique.",
                &r.name
            )));
        }
    }

    Ok(())
}

fn validate_repositories_priority(repositories: &[crate::Repository]) -> crate::Result<()> {
    let mut current_priority = u32::MAX;

    for r in repositories {
        if r.priority > current_priority {
            return Err(crate::Error::Repository(
                "Repositories are not ordered based on their priority.".to_string(),
            ));
        }
        current_priority = r.priority;
    }

    Ok(())
}

fn validate_url(url: &str) -> crate::Result<bool> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(false)
    } else if url.starts_with("file://") {
        Ok(true)
    } else {
        Err(crate::Error::Repository(format!(
            "URL \"{}\" is not valid.",
            url
        )))
    }
}

fn validate_local_repository(repository: &crate::Repository) -> crate::Result<()> {
    if repository.pull_url.is_some() {
        Err(crate::Error::Repository(format!(
            "The local repository \"{}\" may not have a pull_url defined.",
            &repository.name
        )))
    } else if !validate_url(&repository.packet_base_url)? {
        Err(crate::Error::Repository(format!(
            "The local repository \"{}\" must have a file:-url as packet_base_url.",
            &repository.name
        )))
    } else if repository.sources_base_directory.is_none() {
        Err(crate::Error::Repository(format!(
            "The local repository \"{}\" must have a sources_base_directory set.",
            &repository.name
        )))
    } else {
        Ok(())
    }
}

fn validate_remote_repository(repository: &crate::Repository) -> crate::Result<()> {
    if repository.pull_url.is_none() {
        Err(crate::Error::Repository(format!(
            "The remote repository \"{}\" must have a pull_url defined.",
            &repository.name
        )))
    } else if validate_url(&repository.packet_base_url)? {
        Err(crate::Error::Repository(format!(
            "The remote repository \"{}\" must have a http(s):-url as packet_base_url.",
            &repository.name
        )))
    } else if repository.sources_base_directory.is_some() {
        Err(crate::Error::Repository(format!(
            "The remote repository \"{}\" may not have a sources_base_directory set.",
            &repository.name
        )))
    } else {
        Ok(())
    }
}

fn validate_repositories_urls_and_sources(repositories: &[crate::Repository]) -> crate::Result<()> {
    for r in repositories {
        match &r.pull_url {
            None => validate_local_repository(r)?,
            Some(_) => validate_remote_repository(r)?,
        }
    }

    Ok(())
}

fn validate_repositories_dependencies(repositories: &[crate::Repository]) -> crate::Result<()> {
    let known_names: std::collections::HashSet<gng_shared::Name> =
        repositories.iter().map(|r| r.name.clone()).collect();

    for r in repositories {
        for d in &r.dependencies {
            if !known_names.contains(d) {
                return Err(crate::Error::Repository(format!(
                    "Repository \"{}\" has unknown dependency \"{}\".",
                    &r.name, d
                )));
            }
        }
    }

    Ok(())
}

fn validate_repositories(repositories: &[crate::Repository]) -> crate::Result<()> {
    validate_repositories_uniqueness(repositories)?;
    validate_repositories_priority(repositories)?;
    validate_repositories_urls_and_sources(repositories)?;
    validate_repositories_dependencies(repositories)?;

    Ok(())
}

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

        validate_repositories(&repositories)?;

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
        let mut repositories = self.repositories.clone();

        repositories.push(repository_data);
        repositories.sort();

        validate_repositories(&repositories)?;

        self.repositories = repositories;

        backend::write_repositories(&self.db, &self.repositories)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn remove_repository(&mut self, name: &gng_shared::Name) -> crate::Result<()> {
        let mut repositories = self.repositories.clone();

        let uuid = match repositories.iter().position(|r| r.name == *name) {
            None => {
                return Err(crate::Error::Repository(format!(
                    "Repository \"{}\" not found.",
                    name
                )))
            }
            Some(idx) => {
                let uuid = repositories[idx].uuid;
                repositories.remove(idx);
                uuid
            }
        };

        validate_repositories(&repositories)?;

        self.repositories = repositories;
        backend::remove_repository(&self.db, &name.to_string(), &uuid)
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
    use std::convert::{From, TryFrom};

    use super::*;

    fn create_repository_db() -> RepositoryDbImpl {
        let config = sled::Config::default().temporary(true);
        RepositoryDbImpl::new(config.open().expect("Temporary DB should have been valid!"))
            .expect("DB should have been created!")
    }

    fn populate_repository_db(db: &mut RepositoryDbImpl) {
        db.add_repository(crate::Repository {
            name: gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 100,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/base"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: gng_shared::Names::default(),
            tags: gng_shared::Names::from(
                gng_shared::Name::try_from("test1").expect("Name was valid!"),
            ),
        })
        .unwrap();
        db.add_repository(crate::Repository {
            name: gng_shared::Name::try_from("ext_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 1500,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/ext"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: gng_shared::Names::from(
                gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
            ),
            tags: gng_shared::Names::default(),
        })
        .unwrap();
        db.add_repository(crate::Repository {
            name: gng_shared::Name::try_from("tagged_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 1200,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/tagged"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: gng_shared::Names::default(),
            tags: gng_shared::Names::from(
                gng_shared::Name::try_from("test1").expect("Name was valid!"),
            ),
        })
        .unwrap();
        db.add_repository(crate::Repository {
            name: gng_shared::Name::try_from("unrelated_repo").expect("Name was valid!"),
            uuid: crate::Uuid::new_v4(),
            priority: 6000,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/unrelated"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: gng_shared::Names::default(),
            tags: gng_shared::Names::default(),
        })
        .unwrap();
    }

    #[test]
    fn test_repository_validation_ok() {
        let repositories = [
            Repository {
                name: gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: gng_shared::Names::default(),
                tags: gng_shared::Names::from(
                    gng_shared::Name::try_from("test1").expect("Name was valid!"),
                ),
            },
            Repository {
                name: gng_shared::Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: gng_shared::Names::from(
                    gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                ),
                tags: gng_shared::Names::default(),
            },
        ];

        validate_repositories(&repositories).unwrap();
    }

    #[test]
    fn test_repository_validation_duplicate_name() {
        let repositories = [
            Repository {
                name: gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: gng_shared::Names::default(),
                tags: gng_shared::Names::from(
                    gng_shared::Name::try_from("test1").expect("Name was valid!"),
                ),
            },
            Repository {
                name: gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                uuid: crate::Uuid::new_v4(),
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: gng_shared::Names::from(
                    gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                ),
                tags: gng_shared::Names::default(),
            },
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_duplicate_uuid() {
        let uuid = crate::Uuid::new_v4();
        let repositories = [
            Repository {
                name: gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                uuid,
                priority: 10000,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/base"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: gng_shared::Names::default(),
                tags: gng_shared::Names::from(
                    gng_shared::Name::try_from("test1").expect("Name was valid!"),
                ),
            },
            Repository {
                name: gng_shared::Name::try_from("ext_repo").expect("Name was valid!"),
                uuid,
                priority: 1500,
                pull_url: None,
                packet_base_url: String::from("file:///tmp/packets/ext"),
                sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
                dependencies: gng_shared::Names::from(
                    gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
                ),
                tags: gng_shared::Names::default(),
            },
        ];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_validation_unknown_dependency() {
        let uuid = crate::Uuid::new_v4();
        let repositories = [Repository {
            name: gng_shared::Name::try_from("ext_repo").expect("Name was valid!"),
            uuid,
            priority: 1500,
            pull_url: None,
            packet_base_url: String::from("file:///tmp/packets/ext"),
            sources_base_directory: Some(std::path::PathBuf::from("/tmp/sources/base")),
            dependencies: gng_shared::Names::from(
                gng_shared::Name::try_from("base_repo").expect("Name was valid!"),
            ),
            tags: gng_shared::Names::default(),
        }];

        assert!(validate_repositories(&repositories).is_err());
    }

    #[test]
    fn test_repository_setup() {
        let mut repo_db = create_repository_db();
        populate_repository_db(&mut repo_db);

        let repositories = repo_db.list_repositories().unwrap();

        let mut it = repositories.iter();

        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("unrelated_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("ext_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("tagged_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("base_repo")
        );
        assert!(it.next().is_none());
    }
}
