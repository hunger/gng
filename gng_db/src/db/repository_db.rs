// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

use crate::{Error, Repository, Result, Uuid};

use gng_shared::Name;

use std::convert::TryFrom;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn validate_repositories_uniqueness(repositories: &[RepositoryIntern]) -> Result<()> {
    let mut known_names = std::collections::HashSet::new();
    let mut known_uuids = std::collections::HashSet::new();

    for r in repositories {
        let r = r.repository();
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
        Err(Error::Repository(format!("URL \"{}\" is not valid.", url)))
    }
}

fn validate_remote_repository(name: &Name, repository: &crate::RemoteRepository) -> Result<()> {
    if validate_url(&repository.remote_url)? {
        Err(Error::Repository(format!(
            "The remote repository \"{}\" must have a http(s):-url as remote_url.",
            &name
        )))
    } else {
        Ok(())
    }
}

fn validate_repositories_urls_and_sources(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories {
        let r = r.repository();
        if let crate::RepositorySource::Remote(rr) = &r.source {
            validate_remote_repository(&r.name, rr)?;
        }
    }
    Ok(())
}

fn validate_repositories_relations(repositories: &[RepositoryIntern]) -> Result<()> {
    for r in repositories {
        let r = r.repository();
        match &r.relation {
            crate::RepositoryRelation::Dependency(dependencies) => {
                for d in dependencies {
                    if find_repository_by_uuid(repositories, d).is_none() {
                        return Err(Error::Repository(format!(
                            "Repository \"{}\" has unknown dependency \"{}\".",
                            &r.name, &d
                        )));
                    }
                }
            }
            crate::RepositoryRelation::Override(u) => {
                if find_repository_by_uuid(repositories, u).is_none() {
                    return Err(Error::Repository(format!(
                        "Repository \"{}\" overrides unknown repository \"{}\".",
                        &r.name, &u
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_repositories(repositories: &[RepositoryIntern]) -> Result<()> {
    validate_repositories_uniqueness(repositories)?;
    validate_repositories_urls_and_sources(repositories)?;
    validate_repositories_relations(repositories)?;

    Ok(())
}

// ----------------------------------------------------------------------
// - RepositoryIntern:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RepositoryIntern {
    repository: Repository,
    pub search_paths: Vec<crate::Uuid>,
}

impl RepositoryIntern {
    pub const fn new(repository: Repository) -> Self {
        Self {
            repository,
            search_paths: Vec::new(),
        }
    }

    pub const fn repository(&self) -> &Repository {
        &self.repository
    }
}

pub fn find_repository_by_uuid<'a, 'b>(
    repositories: &'a [RepositoryIntern],
    uuid: &'b crate::Uuid,
) -> Option<&'a RepositoryIntern> {
    repositories.iter().find(|r| r.repository().uuid == *uuid)
}

// ----------------------------------------------------------------------
// - RepositoryTreeNode:
// ----------------------------------------------------------------------

struct RepositoryTreeNode<'a> {
    repository: &'a RepositoryIntern,

    overridden_by: Option<usize>,
    overrides: Option<usize>,
    depends_on: Vec<usize>,
    depended_on: Vec<usize>,
}

fn generate_repository_tree(repositories: &[RepositoryIntern]) -> Result<Vec<RepositoryTreeNode>> {
    let uuid_to_index: std::collections::HashMap<Uuid, usize> = repositories
        .iter()
        .enumerate()
        .map(|(idx, r)| (r.repository().uuid, idx))
        .collect();

    let mut tree_nodes = repositories
        .iter()
        .map(|ri| {
            let r = ri.repository();

            let overrides = if let crate::RepositoryRelation::Override(o) = &r.relation {
                Some(*uuid_to_index.get(o).ok_or_else(|| {
                    Error::Repository(format!(
                        "Repository \"{}\" ({}) overrides unknown repository \"{}\"",
                        &r.name, &r.uuid, o
                    ))
                })?)
            } else {
                None
            };

            let depends_on =
                if let crate::RepositoryRelation::Dependency(dependencies) = &r.relation {
                    dependencies
                        .iter()
                        .map(|u| {
                            uuid_to_index.get(u).copied().ok_or_else(|| {
                                Error::Repository(format!(
                                    "Repository \"{}\" ({}) depends on unknown repository \"{}\"",
                                    &r.name, &r.uuid, u
                                ))
                            })
                        })
                        .collect::<Result<Vec<_>>>()?
                } else {
                    Vec::new()
                };

            Ok(RepositoryTreeNode {
                repository: ri,
                overridden_by: None,
                overrides,
                depends_on,
                depended_on: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Second run: Fill in missing data
    for (idx, ri) in repositories.iter().enumerate() {
        let r = ri.repository();

        match &r.relation {
            crate::RepositoryRelation::Override(_) => {
                let o_idx = tree_nodes[idx]
                    .overrides
                    .expect("Must be valid for this kind of node!");

                let other = &mut tree_nodes[o_idx];
                if let Some(overridden_by) = other.overridden_by {
                    return Err(Error::Repository(format!(
                        "Repository \"{}\" ({}) is overridden by repositories \"{}\" ({}) and \"{}\" ({}).",
                        &other.repository.repository().name,
                        &other.repository.repository().uuid,
                        &r.name,
                        &r.uuid,
                        &tree_nodes[overridden_by].repository.repository().name,
                        &tree_nodes[overridden_by].repository.repository().uuid
                    )));
                }
                other.overridden_by = Some(idx);
            }
            crate::RepositoryRelation::Dependency(_) => {
                // Map depends_on to base nodes of overrides:
                let depends_on: Vec<_> = tree_nodes[idx]
                    .depends_on
                    .iter()
                    .map(|d_idx| {
                        let mut idx = *d_idx;
                        while let Some(override_idx) = tree_nodes[idx].overrides {
                            idx = override_idx;
                        }
                        idx
                    })
                    .collect();

                for d in &depends_on {
                    tree_nodes[*d].depended_on.push(idx);
                }

                tree_nodes[idx].depends_on = depends_on;
            }
        }
    }

    assert_eq!(tree_nodes.len(), repositories.len());

    Ok(tree_nodes)
}

#[allow(clippy::needless_collect)]
fn deduplicate_uuids_in_search_path(input: Vec<Uuid>) -> Vec<Uuid> {
    let mut seen_uuids = std::collections::HashSet::new();

    let filtered: Vec<_> = input
        .into_iter()
        .rev()
        .filter(|u| seen_uuids.insert(*u))
        .collect(); // This collect is necessary, otherwise the `rev` calls will cancel out

    filtered.into_iter().rev().collect()
}

fn report_loop(inputs: &[Uuid], node_path: &mut std::collections::HashSet<Uuid>) -> Result<()> {
    if inputs.iter().any(|i| !node_path.insert(*i)) {
        Err(Error::Repository("Relation loop detected.".to_string()))
    } else {
        Ok(())
    }
}

fn calculate_repository_search_path_for_node(
    nodes: &[RepositoryTreeNode],
    current_index: usize,
    parent_node_path: &mut std::collections::HashSet<Uuid>,
    mut result: Vec<Vec<Uuid>>,
) -> Result<Vec<Vec<Uuid>>> {
    assert!(result[current_index].is_empty());

    let mut node_result = Vec::new();

    // Handle overrides:
    let mut overridden_by = Some(current_index); // Add self first!
    let mut override_node_path = std::collections::HashSet::new();
    while let Some(idx) = overridden_by {
        let dest_repo = nodes[idx].repository.repository();
        let uuid = dest_repo.uuid;

        report_loop(&[uuid], &mut override_node_path).map_err(|_| {
            Error::Repository(format!(
                "Repository \"{}\" has override loop.",
                &dest_repo.name,
            ))
        })?;

        node_result.push(uuid);
        overridden_by = nodes[idx].overridden_by;
    }
    node_result = node_result.into_iter().rev().collect(); // Last override comes first!

    // Handle Dependencies:
    for dependency_index in &nodes[current_index].depends_on {
        let dependency_index = *dependency_index;
        let dest_repo = nodes[dependency_index].repository.repository();
        let mut dependency_node_path = parent_node_path.clone();

        if result[dependency_index].is_empty() {
            report_loop(&result[dependency_index], &mut dependency_node_path).map_err(|_| {
                Error::Repository(format!(
                    "Repository \"{}\" has override loop.",
                    &dest_repo.name,
                ))
            })?;

            result = calculate_repository_search_path_for_node(
                nodes,
                dependency_index,
                &mut dependency_node_path,
                result,
            )?;
        }

        node_result.extend_from_slice(&result[dependency_index][..]);
    }

    let node_result = deduplicate_uuids_in_search_path(node_result);

    // Fill in search path into overriding nodes
    let mut overridden_by = nodes[current_index].overridden_by;
    while let Some(idx) = overridden_by {
        result[idx] = node_result.clone();
        overridden_by = nodes[idx].overridden_by;
    }
    // ... and the main node!
    result[current_index] = node_result;

    Ok(result)
}

fn calculate_repository_search_paths(
    repositories: &[RepositoryIntern],
) -> Result<(Vec<Vec<Uuid>>, Vec<Uuid>)> {
    let nodes = generate_repository_tree(repositories)?;

    let mut result = vec![Vec::new(); repositories.len()];
    let mut global_search_path = Vec::new();

    // Find leaf nodes:
    let leaf_indices = {
        let mut tmp: Vec<_> = nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, n)| {
                if n.depended_on.is_empty() && n.overrides.is_none() {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        tmp.sort_by(|a, b| {
            let a = repositories[*a].repository();
            let b = repositories[*b].repository();

            b.priority.cmp(&a.priority)
        });

        tmp
    };

    for l in leaf_indices {
        result = calculate_repository_search_path_for_node(
            &nodes,
            l,
            &mut std::collections::HashSet::new(),
            result,
        )?;
        global_search_path.extend_from_slice(&result[l][..]);
    }

    if result.iter().any(Vec::is_empty) {
        return Err(Error::Repository(
            "Failed to fill in search paths.".to_string(),
        ));
    }
    assert_eq!(result.len(), repositories.len());

    Ok((result, deduplicate_uuids_in_search_path(global_search_path)))
}

fn update_repository_search_paths(repositories: &[RepositoryIntern]) -> Result<Vec<Uuid>> {
    validate_repositories(repositories)?;

    let mut repositories = Vec::from(repositories);
    let (search_paths, global_repository_search_path) =
        calculate_repository_search_paths(&repositories[..])?;

    for (idx, r) in repositories.iter_mut().enumerate() {
        let sp = search_paths[idx].clone();
        assert!(!sp.is_empty());
        r.search_paths = sp;
    }

    assert!(!global_repository_search_path.is_empty());
    Ok(global_repository_search_path)
}

// ----------------------------------------------------------------------
// - RepositoryDb:
// ----------------------------------------------------------------------

/// A `Db` of gng `Packet`s and related information
#[derive(Clone, Debug)]
pub struct RepositoryDb {
    repositories: Vec<RepositoryIntern>,
    global_repository_search_path: Vec<Uuid>,
}

impl RepositoryDb {
    #[tracing::instrument(level = "trace")]
    pub fn reset_db(repositories: &[Repository]) -> Result<Self> {
        let repositories: Vec<_> = repositories
            .iter()
            .map(|r| RepositoryIntern::new(r.clone()))
            .collect();

        let global_repository_search_path = update_repository_search_paths(&repositories)?;

        Ok(Self {
            repositories,
            global_repository_search_path,
        })
    }

    /// Resolve some user provided `&str` to an `Repository` `Uuid`.
    pub fn resolve_repository(&self, input: &str) -> Option<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(input) {
            find_repository_by_uuid(&self.repositories, &uuid).map(|_| uuid)
        } else if let Ok(name) = Name::try_from(input) {
            self.repositories
                .iter()
                .find(|r| r.repository().name == name)
                .map(|ri| ri.repository().uuid)
        } else {
            None
        }
    }

    /// Get the search path for a `Repository`.
    pub fn search_path(&self, uuid: Option<&Uuid>) -> Result<Vec<Uuid>> {
        match uuid {
            None => Ok(self.global_repository_search_path.clone()),
            Some(uuid) => {
                if let Some(repository) = find_repository_by_uuid(&self.repositories, uuid) {
                    Ok(repository.search_paths.clone())
                } else {
                    Err(Error::Repository(format!(
                        "Could not find repository with UUID {}.",
                        uuid
                    )))
                }
            }
        }
    }

    /// Get a `Repository`.
    pub fn repository(&self, uuid: &Uuid) -> Result<Repository> {
        if let Some(repository) = find_repository_by_uuid(&self.repositories, uuid) {
            Ok(repository.repository().clone())
        } else {
            Err(Error::Repository(format!(
                "Could not find repository with UUID {}.",
                uuid
            )))
        }
    }

    pub fn all_repositories(&self) -> Vec<Repository> {
        self.global_repository_search_path
            .iter()
            .map(|u| {
                find_repository_by_uuid(&self.repositories, u)
                    .expect("Must exists!")
                    .repository()
                    .clone()
            })
            .collect()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn add_repository(&mut self, repository_data: Repository) -> Result<()> {
        let mut repositories = self.repositories.clone();

        repositories.push(RepositoryIntern::new(repository_data));
        let global_repository_search_path = update_repository_search_paths(&repositories)?;

        self.repositories = repositories;
        self.global_repository_search_path = global_repository_search_path;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn remove_repository(&mut self, uuid: &Uuid) -> Result<()> {
        let mut to_remove = Vec::new();
        let repositories: Vec<_> = self
            .repositories
            .iter()
            .filter(|r| {
                if r.repository().uuid == *uuid {
                    false
                } else {
                    to_remove.push(r.repository().name.clone());
                    true
                }
            })
            .cloned()
            .collect();
        if repositories.len() == self.repositories.len() {
            return Err(Error::Repository(format!(
                "Repository \"{}\" not found, can not remove.",
                uuid
            )));
        }

        let global_repository_search_path = update_repository_search_paths(&repositories)?;

        self.repositories = repositories;
        self.global_repository_search_path = global_repository_search_path;

        Ok(())
    }

    pub fn fsck(&self) -> Result<bool> {
        update_repository_search_paths(&self.repositories)?;

        Ok(true)
    }
}

impl Default for RepositoryDb {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        Self {
            repositories: Vec::new(),
            global_repository_search_path: Vec::new(),
        }
    }
} // Default for DbImpl

#[cfg(test)]
mod tests {
    use std::convert::{From, TryFrom};

    use crate::Repository;

    use super::*;

    fn populate_repository_db(db: &mut RepositoryDb) {
        db.add_repository(Repository {
            name: Name::try_from("base_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 100,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![]),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("ext_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 1500,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![db
                .resolve_repository("base_repo")
                .expect("Repo was valid!")]),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("tagged_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 1200,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/tagged"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![]),
        })
        .unwrap();
        db.add_repository(Repository {
            name: Name::try_from("unrelated_repo").expect("Name was valid!"),
            uuid: Uuid::new_v4(),
            priority: 6000,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/unrelated"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![]),
        })
        .unwrap();
    }

    #[test]
    fn repository_validation_ok() {
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
            }),
        ];

        update_repository_search_paths(&repositories).unwrap();
    }

    #[test]
    fn repository_validation_duplicate_name() {
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: Uuid::new_v4(),
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
            }),
        ];

        assert!(update_repository_search_paths(&repositories).is_err());
    }

    #[test]
    fn repository_validation_duplicate_uuid() {
        let uuid = Uuid::new_v4();
        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![]),
            }),
        ];

        assert!(update_repository_search_paths(&repositories).is_err());
    }

    #[test]
    fn repository_validation_dependency_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: uuid2,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![uuid1]),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: uuid1,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Dependency(vec![uuid2]),
            }),
        ];

        assert!(update_repository_search_paths(&repositories).is_err());
    }

    #[test]
    fn repository_validation_override_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [
            RepositoryIntern::new(Repository {
                name: Name::try_from("base_repo").expect("Name was valid!"),
                uuid: uuid2,
                priority: 100,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Override(uuid1),
            }),
            RepositoryIntern::new(Repository {
                name: Name::try_from("ext_repo").expect("Name was valid!"),
                uuid: uuid1,
                priority: 1500,
                source: crate::RepositorySource::Local(crate::LocalRepository {
                    sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/ext"),
                    export_directory: None,
                }),
                relation: crate::RepositoryRelation::Override(uuid2),
            }),
        ];

        assert!(update_repository_search_paths(&repositories).is_err());
    }

    #[test]
    fn repository_validation_unknown_dependency() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let repositories = [RepositoryIntern::new(Repository {
            name: Name::try_from("base_repo").expect("Name was valid!"),
            uuid: uuid1,
            priority: 100,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![uuid2]),
        })];

        assert!(update_repository_search_paths(&repositories).is_err());
    }

    fn create_dependent_repo(name: &str, uuid: &Uuid, dependencies: Vec<Uuid>) -> RepositoryIntern {
        RepositoryIntern::new(Repository {
            name: Name::try_from(name).expect("Name was valid!"),
            uuid: *uuid,
            priority: 1500,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from(format!(
                    "file:///tmp/sources/{}",
                    &name
                )),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(dependencies),
        })
    }

    fn create_override_repo(name: &str, uuid: &Uuid, overrides: Uuid) -> RepositoryIntern {
        RepositoryIntern::new(Repository {
            name: Name::try_from(name).expect("Name was valid!"),
            uuid: *uuid,
            priority: 1500,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from(format!(
                    "file:///tmp/sources/{}",
                    &name
                )),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Override(overrides),
        })
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

        let repositories = [
            create_dependent_repo("r3", &uuid_3, vec![uuid_2]),
            create_override_repo("r1o0", &uuid_1o0, uuid_1),
            create_override_repo("r2o1", &uuid_2o1, uuid_2o0),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0]),
            create_override_repo("r2o0", &uuid_2o0, uuid_2),
            create_dependent_repo("r0", &uuid_0, vec![]),
            create_dependent_repo("r2", &uuid_2, vec![uuid_1]),
        ];

        let global_search_path = update_repository_search_paths(&repositories)
            .expect("Input was supposed to be correct");

        assert_eq!(
            global_search_path,
            vec![uuid_3, uuid_2o1, uuid_2o0, uuid_2, uuid_1o0, uuid_1, uuid_0]
        )
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
            create_dependent_repo("r0", &uuid_0, vec![]),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0]),
            create_dependent_repo("r2l0", &uuid_2left0, vec![uuid_1]),
            create_dependent_repo("r2l1", &uuid_2left1, vec![uuid_2left0]),
            create_dependent_repo("r2r0", &uuid_2right0, vec![uuid_1]),
            create_override_repo("r2r0o0", &uuid_2right0o0, uuid_2right0),
            create_dependent_repo("r3", &uuid_3, vec![uuid_2left1, uuid_2right0]),
        ];

        let global_search_path = update_repository_search_paths(&repositories)
            .expect("Input was supposed to be correct");

        assert_eq!(
            global_search_path,
            vec![
                uuid_3,
                uuid_2left1,
                uuid_2left0,
                uuid_2right0o0,
                uuid_2right0,
                uuid_1,
                uuid_0,
            ]
        )
    }

    #[test]
    fn repository_setup() {
        let mut repo_db = RepositoryDb::default();
        populate_repository_db(&mut repo_db);

        let repositories = repo_db.all_repositories();

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
            String::from("base_repo")
        );
        assert_eq!(
            it.next().unwrap().name.to_string(),
            String::from("tagged_repo")
        );
        assert!(it.next().is_none());
    }
}
