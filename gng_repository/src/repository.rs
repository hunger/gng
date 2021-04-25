// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

mod definitions;

use definitions::RepositoryInternalData;

// ----------------------------------------------------------------------
// - Repository:
// ----------------------------------------------------------------------

/// A `Repository` of gng `Packet`s and related information
pub trait Repository {
    /// Get the Schema version
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn version(&self) -> crate::Result<u32>;

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn list_repositories(&self) -> crate::Result<Vec<crate::RepositoryData>>;

    /// Add a new repository
    ///
    /// # Errors
    /// Any of the crate's `Error`s can be returned from here.
    fn add_repository(&mut self, repository_data: crate::RepositoryData) -> crate::Result<()>;

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
// - RepositoryImpl:
// ----------------------------------------------------------------------

pub(crate) struct RepositoryImpl {
    db: sled::Db,
    repositories: Vec<crate::RepositoryData>,
}

impl RepositoryImpl {
    const MAGIC: &'static str = "\"1e925d91-3294-4676-add6-917376d89d58\"";
    const MAGIC_KEY: &'static str = "magic";

    const VERSION: u32 = 1;
    const VERSION_KEY: &'static str = "schema_version";

    const REPOSITORIES_TREE: &'static str = "repositories";
    const PACKETS_TREE: &'static str = "packets";
    const CONTENTS_TREE: &'static str = "contents";

    #[tracing::instrument(level = "trace")]
    pub(crate) fn new(path: &std::path::Path) -> crate::Result<Self> {
        let config = sled::Config::default().path(path.to_owned());

        let mut result = Self {
            db: config.open()?,
            repositories: Vec::new(),
        };

        match result.db.get(Self::MAGIC_KEY)? {
            Some(v) if v == Self::MAGIC => result.db_validate()?,
            Some(_) => return Err(crate::Error::WrongMagic),
            None => result.db_setup()?,
        };

        result.repositories = result.read_repositories()?;

        Ok(result)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn read_repositories(&mut self) -> crate::Result<Vec<crate::RepositoryData>> {
        let tree = self.db.open_tree(Self::REPOSITORIES_TREE)?;

        let (data, id_map) = {
            let mut data = Vec::with_capacity(tree.len());
            let mut id_map = std::collections::HashMap::with_capacity(tree.len());

            for tree_result in tree.iter() {
                match tree_result {
                    Err(e) => return Err(e.into()),
                    Ok((k, v)) => {
                        let name: gng_shared::Name = serde_json::from_slice(&k[..])
                            .map_err(|_| crate::Error::WrongSchema)?;
                        let rid: RepositoryInternalData = serde_json::from_reader(&v[..])
                            .map_err(|_| crate::Error::WrongSchema)?;

                        id_map.insert(rid.uuid, name.clone());

                        data.push((name, rid));
                    }
                }
            }
            (data, id_map)
        };

        let mut result = data
            .iter()
            .map(|(n, d)| -> crate::Result<crate::RepositoryData> {
                let dependencies = gng_shared::Names::from(
                    d.dependencies
                        .iter()
                        .map(|u| {
                            let name = id_map.get(u).ok_or_else(|| {
                                crate::Error::UnknownRepositoryDependency(u.to_string(), n.clone())
                            })?;
                            Ok(name.clone())
                        })
                        .collect::<crate::Result<Vec<gng_shared::Name>>>()?,
                );

                Ok(crate::RepositoryData {
                    name: n.clone(),
                    uuid: d.uuid,
                    priority: d.priority,
                    pull_url: d.pull_url.clone(),
                    packet_base_url: d.packet_base_url.clone(),
                    sources_base_directory: d.sources_base_directory.clone(),
                    dependencies,
                })
            })
            .collect::<crate::Result<Vec<crate::RepositoryData>>>()?;

        result.sort();

        Ok(result)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn write_repositories(&mut self) -> crate::Result<()> {
        let repositories = &self.repositories[..];

        let tree = self.db.open_tree(Self::REPOSITORIES_TREE)?;

        let name_map: std::collections::HashMap<gng_shared::Name, crate::Uuid> = repositories
            .iter()
            .map(|r| (r.name.clone(), r.uuid))
            .collect();

        let batch = {
            let mut batch = sled::Batch::default();

            for r in repositories {
                batch.insert(
                    serde_json::to_vec(&r.name).expect("names must be convertible to JSON"),
                    serde_json::to_vec(&definitions::RepositoryInternalData {
                        uuid: r.uuid,
                        priority: r.priority,
                        pull_url: r.pull_url.clone(),
                        packet_base_url: r.packet_base_url.clone(),
                        sources_base_directory: r.sources_base_directory.clone(),
                        dependencies: r
                            .dependencies
                            .into_iter()
                            .map(|n| {
                                name_map.get(n).cloned().ok_or_else(|| {
                                    crate::Error::UnknownRepositoryDependency(
                                        n.to_string(),
                                        r.name.clone(),
                                    )
                                })
                            })
                            .collect::<crate::Result<Vec<crate::Uuid>>>()?,
                    })
                    .expect("Data structure must be convertible to JSON"),
                )
            }

            batch
        };

        tree.apply_batch(batch)?;

        tree.flush()?;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn db_setup(&mut self) -> crate::Result<()> {
        self.db.insert(
            Self::VERSION_KEY,
            &serde_json::to_vec(&Self::VERSION).map_err(|_| crate::Error::WrongSchema)?[..],
        )?;

        let _repository_tree = self.db.open_tree(Self::REPOSITORIES_TREE)?;
        let _packages_tree = self.db.open_tree(Self::PACKETS_TREE)?;
        let _contents_tree = self.db.open_tree(Self::CONTENTS_TREE)?;

        self.db.insert(Self::MAGIC_KEY, Self::MAGIC)?;

        self.db.flush()?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn db_validate(&mut self) -> crate::Result<()> {
        match self.version()? {
            v if v < Self::VERSION => Err(crate::Error::WrongSchema),
            v if v == Self::VERSION => Ok(()),
            _ => Err(crate::Error::WrongSchema),
        }
    }
}

impl Repository for RepositoryImpl {
    #[tracing::instrument(level = "trace", skip(self))]
    fn version(&self) -> crate::Result<u32> {
        let version = self
            .db
            .get(Self::VERSION_KEY)?
            .expect("Version must be set!");
        serde_json::from_slice(&version[..]).map_err(|_| crate::Error::WrongSchema)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn list_repositories(&self) -> crate::Result<Vec<crate::RepositoryData>> {
        Ok(self.repositories.clone())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn add_repository(&mut self, repository_data: crate::RepositoryData) -> crate::Result<()> {
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

        self.write_repositories()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn remove_repository(&mut self, name: &gng_shared::Name) -> crate::Result<()> {
        let mut to_remove = self.repositories.len();
        for (idx, r) in self.repositories.iter().enumerate() {
            if &r.name == name {
                to_remove = idx;
            } else if r.dependencies.contains(name) {
                return Err(crate::Error::RepositoryInUse {
                    used_repository: name.clone(),
                    using_repository: r.name.clone(),
                });
            }
        }

        if to_remove == self.repositories.len() {
            return Err(crate::Error::UnknownRepository(name.clone()));
        }

        self.repositories.remove(to_remove);

        let tree = self.db.open_tree(Self::REPOSITORIES_TREE)?;
        tree.remove(&serde_json::to_vec(&name).expect("Names can be serialized!")[..])?;

        tree.flush()?;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn fsck(&self) -> crate::Result<bool> {
        Ok(true)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn dump_metadata(&mut self) -> crate::Result<()> {
        println!("Metadata:");
        for data in self.db.iter() {
            match data {
                Err(e) => println!("Error: {}", e),
                Ok((k, v)) => {
                    let key = std::string::String::from_utf8((&k[..]).to_vec())
                        .unwrap_or(format!("{:?}", k));
                    let value = if v.is_ascii() {
                        std::string::String::from_utf8((&v[..]).to_vec())
                            .expect("I though this is ASCII?!")
                    } else {
                        format!("{:?}", v)
                    };
                    println!("    \"{}\" => {}.", key, value,)
                }
            }
        }

        println!("\nKnown trees:");
        for n in &self.db.tree_names() {
            let n = std::string::String::from_utf8((&n[..]).to_vec()).unwrap_or(format!("{:?}", n));
            if n.starts_with("__") {
                continue;
            }
            println!("    \"{}\"", &n)
        }
        Ok(())
    }
}
