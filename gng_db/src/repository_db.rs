// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{deduplicate, Error, LocalRepository, Repository, RepositoryNode, Result, Uuid};

use gng_shared::Name;

use std::convert::TryFrom;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn validate_repositories_uniqueness(repositories: &[RepositoryData]) -> Result<()> {
    let mut known_names = std::collections::HashSet::new();
    let mut known_uuids = std::collections::HashSet::new();

    for r in repositories {
        if !known_names.insert(r.name.clone()) {
            return Err(Error::Repository(format!(
                "Repository name \"{}\" is not unique.",
                &r.name
            )));
        }
        if !known_uuids.insert(r.uuid) {
            return Err(Error::Repository(format!(
                "Repository UUID \"{}\" is not unique.",
                &r.name
            )));
        }
    }

    Ok(())
}

fn validate_url(url: &str) -> Result<bool> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(false)
    } else if url.starts_with("file://") {
        Ok(true)
    } else {
        Err(Error::Repository(format!(
            "URL \"{}\" is not allowed.",
            url
        )))
    }
}

fn validate_repositories_urls_and_sources(repositories: &[RepositoryData]) -> Result<()> {
    let mut local_paths: Vec<&std::path::PathBuf> = Vec::new();
    let mut remote_urls: Vec<&String> = Vec::new();

    for r in repositories {
        match &r.source {
            crate::RepositorySource::Local(lr) => {
                let sbd = &lr.sources_base_directory;
                for p in &local_paths {
                    if p.starts_with(sbd) || sbd.starts_with(p) {
                        return Err(Error::Repository(format!(
                            "Repository \"{}\" ({}) has a duplicate sources_base_directory.",
                            &r.name, &r.uuid,
                        )));
                    }
                }
                local_paths.push(sbd);
            }
            crate::RepositorySource::Remote(rr) => {
                let ru = &rr.remote_url;
                for prev_url in &remote_urls {
                    if prev_url == &ru {
                        return Err(Error::Repository(format!(
                            "Repository \"{}\" ({}) has a duplicate remote_url.",
                            &r.name, &r.uuid,
                        )));
                    }
                }
                remote_urls.push(ru);

                validate_url(ru)?;
            }
        }
    }
    Ok(())
}

fn validate_repositories(repositories: &[RepositoryData]) -> Result<()> {
    validate_repositories_uniqueness(repositories)?;
    validate_repositories_urls_and_sources(repositories)?;

    Ok(())
}

fn compare_repositories(a: &Repository, b: &Repository) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match a.priority.cmp(&b.priority) {
        Ordering::Less => Ordering::Greater,
        Ordering::Equal => a.uuid.cmp(&b.uuid),
        Ordering::Greater => Ordering::Less,
    }
}

fn create_placeholder_repository() -> crate::RepositoryNode {
    std::rc::Rc::new(Repository {
        name: Name::try_from("placeholder").expect("Name was valid."),
        uuid: Uuid::new_v4(),
        priority: 0,
        source: crate::RepositorySource::Local(LocalRepository {
            sources_base_directory: std::path::PathBuf::new(),
            export_directory: None,
        }),
        overridden_by: Vec::new(),
        overrides: None,
        depends_on: Vec::new(),
        depended_on: Vec::new(),
    })
}

fn generate_uuid_repository_map(
    repositories: &[RepositoryData],
) -> std::collections::HashMap<Uuid, RepositoryNode> {
    let placeholder = create_placeholder_repository();

    repositories
        .iter()
        .map(|r| {
            let repo = std::rc::Rc::new(Repository {
                name: r.name.clone(),
                uuid: r.uuid,
                source: r.source.clone(),
                priority: r.priority,
                overridden_by: Vec::new(),
                overrides: if r.is_override() {
                    Some(placeholder.clone())
                } else {
                    None
                },
                depends_on: Vec::new(),
                depended_on: Vec::new(),
            });

            (r.uuid, repo)
        })
        .collect::<std::collections::HashMap<_, _, _>>()
}

fn generate_repository_nodes_with_relations(
    repositories: &[RepositoryData],
    uuid_to_repository: &std::collections::HashMap<Uuid, RepositoryNode>,
) -> Result<Vec<RepositoryNode>> {
    repositories
        .iter()
        .map(|r| {
            let mut repo = uuid_to_repository[&r.uuid].clone();

            let overrides = if let RepositoryRelation::Override(o) = &r.relation {
                let mut other_repo = uuid_to_repository.get(o).ok_or_else(|| {
                    Error::Repository(format!(
                        "Repository \"{}\" ({}) overrides unknown repository \"{}\"", &r.name, &r.uuid, o
                    ))
                })?.clone();

                if other_repo.is_override() {
                    return Err(
                        Error::Repository(format!(
                            "Repository \"{}\" ({}) overrides another override repository. This is not allowed!", &r.name, &r.uuid,
                        ))
                    );
                }

                unsafe { std::rc::Rc::get_mut_unchecked(&mut other_repo).overridden_by.push(repo.clone()); }

                Some(other_repo)
            } else {
                None
            };
            unsafe { std::rc::Rc::get_mut_unchecked(&mut repo).overrides = overrides; }

            let depends_on = if let RepositoryRelation::Dependency(dependencies) = &r.relation {
                dependencies
                    .iter()
                    .map(|d| {
                        let mut other_repo = uuid_to_repository.get(d).ok_or_else(|| {
                            Error::Repository(format!(
                                "Repository \"{}\" ({}) depends on unknown repository \"{}\"", &r.name, &r.uuid, d
                            ))
                        })?.clone();

                        if other_repo.is_override() {
                            return Err(
                                Error::Repository(format!(
                                    "Repository \"{}\" ({}) depends on an override repository. This is not allowed!", &r.name, &r.uuid,
                                ))
                            );
                        }

                        if other_repo.depends_on_repository(&repo.uuid) {
                            return Err(
                                Error::Repository(format!(
                                    "Repository \"{}\" ({}) produces a dependency loop!",
                                &r.name, &r.uuid,
                                ))
                            );
                        }

                        unsafe { std::rc::Rc::get_mut_unchecked(&mut other_repo).depended_on.push(repo.clone()); }

                        Ok(other_repo)
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            unsafe { std::rc::Rc::get_mut_unchecked(&mut repo).depends_on = depends_on; }

            Ok(repo)
        })
        .collect()
}

fn find_leaf_repository_nodes(repository_nodes: &[RepositoryNode]) -> Vec<RepositoryNode> {
    let mut leaves: Vec<_> = repository_nodes
        .iter()
        .filter(|r| !r.is_override() && r.depended_on.is_empty())
        .cloned()
        .collect();

    leaves.sort_by(|a, b| compare_repositories(a, b));

    leaves
}

fn generate_repository_tree(repositories: &[RepositoryData]) -> Result<Vec<crate::RepositoryNode>> {
    validate_repositories(repositories)?;

    // 0. run: Create empty Repositories (mapped by UUID)
    let uuid_to_repository = generate_uuid_repository_map(repositories);

    // 1. run: Fill information from RepositoryData into Repositories:
    let mut repository_nodes =
        generate_repository_nodes_with_relations(repositories, &uuid_to_repository)?;

    // 2. step: Sort overrides and depends_on based on priority:
    for rn in &mut repository_nodes {
        unsafe {
            std::rc::Rc::get_mut_unchecked(rn)
                .overridden_by
                .sort_by(|a, b| compare_repositories(a, b));
        }
        unsafe {
            std::rc::Rc::get_mut_unchecked(rn)
                .depends_on
                .sort_by(|a, b| compare_repositories(a, b));
        }
        unsafe {
            // This is not required, but makes sure our nodes are more stable:
            std::rc::Rc::get_mut_unchecked(rn)
                .depended_on
                .sort_by(|a, b| compare_repositories(a, b));
        }
    }

    assert_eq!(repository_nodes.len(), repositories.len());

    // Make sure to return results in a defined order:
    repository_nodes.sort_by(|a, b| compare_repositories(a, b));

    Ok(repository_nodes)
}

// ----------------------------------------------------------------------
// - RepositoryData:
// ----------------------------------------------------------------------

/// The relations between a `Repository` and other `Repository`s.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum RepositoryRelation {
    /// Override another `Repository`
    #[serde(rename = "override")]
    Override(Uuid),
    /// Depend on zero or more other `Repository`s.
    #[serde(rename = "dependencies")]
    Dependency(Vec<Uuid>),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
/// Data on a repository of `Packet`s.
pub struct RepositoryData {
    /// The user-visible name of this repository
    pub name: gng_shared::Name,
    /// The repository UUID
    pub uuid: Uuid,
    /// The priority of this `RepositoryData`
    #[serde(default)]
    pub priority: u32,

    /// `Repository`(s) this one relates to
    #[serde(flatten)]
    pub relation: RepositoryRelation,

    /// The `RepositoryConnectivity` we are dealing with plus all
    /// the kind-specific data.
    /// Basically: Where does all the data in this `Repository` come from?
    #[serde(flatten)]
    pub source: crate::RepositorySource,
}

impl RepositoryData {
    /// Is this a local repository?
    #[must_use]
    pub const fn is_local(&self) -> bool {
        matches!(self.source, crate::RepositorySource::Local(_))
    }

    /// Does this repository override some other repository?
    #[must_use]
    pub const fn is_override(&self) -> bool {
        matches!(self.relation, RepositoryRelation::Override(_))
    }
}

impl PartialEq for RepositoryData {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for RepositoryData {}

// ----------------------------------------------------------------------
// - RepositoryDb:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
#[derive(Clone, Debug)]
pub struct RepositoryDb {
    repositories: Vec<crate::RepositoryNode>,
    leaves: Vec<crate::RepositoryNode>,
}

impl RepositoryDb {
    /// Create a new `RepositoryDb` populated with all repositories found in `repository_directory`.
    ///
    /// # Errors
    /// Return `Error::Repository` if there are inconsistencies discovered while loading the repositories
    pub fn open(repository_directory: &std::path::Path) -> Result<Self> {
        let repositories = backend::read_repositories(repository_directory)?;
        let repositories = generate_repository_tree(&repositories[..])?;

        tracing::info!(
            "Repository DB with {} repositories created from {:?}.",
            &repositories.len(),
            repository_directory.display(),
        );

        let leaves = find_leaf_repository_nodes(&repositories);

        Ok(Self {
            repositories,
            leaves,
        })
    }

    /// Resolve some user provided `&str` to an `Repository` `Uuid`.
    #[must_use]
    pub fn resolve_repository(&self, input: &str) -> Option<RepositoryNode> {
        if let Ok(uuid) = Uuid::parse_str(input) {
            if let Some(r) = self.repositories.iter().find(|r| r.uuid == uuid).cloned() {
                return Some(r);
            }
        }
        if let Ok(name) = Name::try_from(input) {
            return self.repositories.iter().find(|r| r.name == name).cloned();
        }
        None
    }

    /// Find a `Repository` that will adopt packets from a specific `Packet` source directory.
    #[must_use]
    pub fn repository_for_packet_source_path(
        &self,
        input: &std::path::Path,
    ) -> Option<RepositoryNode> {
        self.repositories
            .iter()
            .find(|r| match &r.source {
                crate::RepositorySource::Local(lr) => input.starts_with(&lr.sources_base_directory),
                crate::RepositorySource::Remote(_) => false,
            })
            .cloned()
    }

    /// Get the search path for a `Repository`.
    /// A `uuid` of `None` will return the global search path
    #[must_use]
    pub fn search_path(&self) -> Vec<Uuid> {
        deduplicate(self.leaves.iter().flat_map(|r| r.search_path()).collect())
    }

    /// List all repositories
    #[must_use]
    pub fn all_repositories(&self) -> Vec<RepositoryNode> {
        self.repositories.clone()
    }

    /// List leaf repositories
    #[must_use]
    pub fn leaf_repositories(&self) -> Vec<RepositoryNode> {
        self.leaves.clone()
    }

    // /// Sanity check all known repositories
    // ///
    // /// # Errors
    // /// `Error::Repository` might be returned if inconsistencies are detected.
    // pub fn fsck(&self) -> Result<bool> {
    //     let mut repositories = self.repositories.clone();
    //     update_repository_search_paths(&mut repositories)?;

    //     Ok(true)
    // }
}

impl Default for RepositoryDb {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        tracing::info!("Repository DB with 0 repositories created.",);

        Self {
            repositories: Vec::new(),
            leaves: Vec::new(),
        }
    }
}

// ----------------------------------------------------------------------
// - Backend:
// ----------------------------------------------------------------------

mod backend {
    use crate::{Error, Result};

    use super::RepositoryData;

    pub fn read_repositories(
        repository_directory: &std::path::Path,
    ) -> Result<Vec<RepositoryData>> {
        let repo_file_extension = std::ffi::OsStr::new("conf");

        let mut result = Vec::new();

        for f in repository_directory.read_dir()? {
            let f_path = f?.path();
            if !f_path.is_file() {
                tracing::trace!("    Skipping {}: Not a file.", f_path.display());
                continue;
            }
            if let Some(extension) = f_path.extension() {
                if extension == repo_file_extension {
                    let file = std::fs::File::open(&f_path)?;

                    let repo =
                        serde_json::from_reader(std::io::BufReader::new(file)).map_err(|e| {
                            println!("Original error: {}.", e);
                            Error::Repository(format!(
                                "Could not read repository from {}.",
                                &f_path.display()
                            ))
                        })?;

                    tracing::trace!("    Read {} -> {:?}.", f_path.display(), &repo);

                    result.push(repo);
                    continue;
                }
                tracing::trace!(
                    "    Skipping {}: Extension is not \".{}\".",
                    f_path.display(),
                    &repo_file_extension.to_string_lossy()
                );
                continue;
            }
            tracing::trace!("    Skipping {}: No file extension.", f_path.display());
        }

        Ok(result)
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::convert::{From, TryFrom};

    use super::{RepositoryData, RepositoryRelation};

    use super::*;

    #[test]
    fn repository_validation_ok() {
        let repositories = [
            RepositoryData {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![]),
            },
            RepositoryData {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![]),
            },
        ];

        generate_repository_tree(&repositories).unwrap();
    }

    #[test]
    fn repository_validation_duplicate_name() {
        let repositories = [
            RepositoryData {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![]),
            },
            RepositoryData {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![]),
            },
        ];

        assert!(generate_repository_tree(&repositories).is_err());
    }

    #[test]
    fn repository_validation_duplicate_uuid() {
        let uuid = Uuid::new_v4();
        let repositories = [
            RepositoryData {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![]),
            },
            RepositoryData {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![]),
            },
        ];

        assert!(generate_repository_tree(&repositories).is_err());
    }

    #[test]
    fn repository_validation_dependency_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [
            RepositoryData {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: uuid2,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![uuid1]),
            },
            RepositoryData {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: uuid1,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Dependency(vec![uuid2]),
            },
        ];

        assert!(generate_repository_tree(&repositories).is_err());
    }

    #[test]
    fn repository_validation_override_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [
            RepositoryData {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: uuid2,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Override(uuid1),
            },
            RepositoryData {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: uuid1,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: RepositoryRelation::Override(uuid2),
            },
        ];

        assert!(generate_repository_tree(&repositories).is_err());
    }

    #[test]
    fn repository_validation_unknown_dependency() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [RepositoryData {
            name: Name::try_from("base_repo").expect("Name was valid!"),
            uuid: uuid1,
            priority: 100,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                export_directory: None,
            }),
            relation: RepositoryRelation::Dependency(vec![uuid2]),
        }];

        assert!(generate_repository_tree(&repositories).is_err());
    }

    fn create_dependent_repo(
        name: &str,
        uuid: &Uuid,
        dependencies: Vec<Uuid>,
        priority: u32,
    ) -> RepositoryData {
        RepositoryData {
            name: Name::try_from(name).expect("Name was valid!"),
            uuid: *uuid,
            priority,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from(format!(
                    "file:///tmp/sources/{}",
                    &name
                )),
                export_directory: None,
            }),
            relation: RepositoryRelation::Dependency(dependencies),
        }
    }

    fn create_override_repo(
        name: &str,
        uuid: &Uuid,
        overrides: Uuid,
        priority: u32,
    ) -> RepositoryData {
        RepositoryData {
            name: Name::try_from(name).expect("Name was valid!"),
            uuid: *uuid,
            priority,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from(format!(
                    "file:///tmp/sources/{}",
                    &name
                )),
                export_directory: None,
            }),
            relation: RepositoryRelation::Override(overrides),
        }
    }

    #[test]
    fn update_search_paths_line() {
        let uuid_0 = Uuid::new_v4();
        let uuid_1 = Uuid::new_v4();
        let uuid_1o0 = Uuid::new_v4();
        let uuid_2 = Uuid::new_v4();
        let uuid_2o0 = Uuid::new_v4();
        let uuid_2o1 = Uuid::new_v4();
        let uuid_3 = Uuid::new_v4();
        let uuid_3o0 = Uuid::new_v4();

        let repositories = [
            create_dependent_repo("r3", &uuid_3, vec![uuid_2], 1500),
            create_override_repo("r1o0", &uuid_1o0, uuid_1, 10000),
            create_override_repo("r2o1", &uuid_2o1, uuid_2, 2000),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0], 1500),
            create_override_repo("r2o0", &uuid_2o0, uuid_2, 15000),
            create_dependent_repo("r0", &uuid_0, vec![], 1500),
            create_dependent_repo("r2", &uuid_2, vec![uuid_1], 1500),
            create_override_repo("r3o0", &uuid_3o0, uuid_3, 150),
        ];

        let repository_nodes =
            generate_repository_tree(&repositories).expect("Input was supposed to be correct");

        let leaves = find_leaf_repository_nodes(&repository_nodes);
        assert_eq!(
            leaves.iter().map(|r| r.uuid).collect::<Vec<_>>(),
            vec![uuid_3]
        );
        assert_eq!(
            leaves[0].search_path(),
            vec![uuid_3o0, uuid_3, uuid_2o0, uuid_2o1, uuid_2, uuid_1o0, uuid_1, uuid_0]
        );
    }

    #[test]
    fn update_search_paths_diamond() {
        let uuid_0 = Uuid::new_v4();
        let uuid_1 = Uuid::new_v4();
        let uuid_2left0 = Uuid::new_v4();
        let uuid_2left1 = Uuid::new_v4();
        let uuid_2right0 = Uuid::new_v4();
        let uuid_2right0o0 = Uuid::new_v4();
        let uuid_3 = Uuid::new_v4();

        let repositories = [
            create_dependent_repo("r0", &uuid_0, vec![], 1500),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0], 1500),
            create_dependent_repo("r2l0", &uuid_2left0, vec![uuid_1], 1500),
            create_dependent_repo("r2l1", &uuid_2left1, vec![uuid_2left0], 5100),
            create_dependent_repo("r2r0", &uuid_2right0, vec![uuid_1], 1500),
            create_override_repo("r2r0o0", &uuid_2right0o0, uuid_2right0, 99),
            create_dependent_repo("r3", &uuid_3, vec![uuid_2left1, uuid_2right0], 1500),
        ];

        let repository_nodes =
            generate_repository_tree(&repositories).expect("Input was supposed to be correct");

        for rn in &repository_nodes {
            println!("{}", rn.to_pretty_string());
        }

        let leaves = find_leaf_repository_nodes(&repository_nodes);
        assert_eq!(
            leaves.iter().map(|r| r.uuid).collect::<Vec<_>>(),
            vec![uuid_3]
        );

        assert_eq!(
            leaves[0].search_path(),
            vec![
                uuid_3,
                uuid_2left1,
                uuid_2left0,
                uuid_1,
                uuid_0,
                uuid_2right0o0,
                uuid_2right0,
            ]
        )
    }
}
