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
        Err(Error::Repository(format!(
            "URL \"{}\" is not allowed.",
            url
        )))
    }
}

fn validate_repositories_urls_and_sources(repositories: &[RepositoryIntern]) -> Result<()> {
    let mut local_paths: Vec<&std::path::PathBuf> = Vec::new();
    let mut remote_urls: Vec<&String> = Vec::new();

    for r in repositories {
        let r = r.repository();
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

fn validate_repositories(repositories: &[RepositoryIntern]) -> Result<()> {
    validate_repositories_uniqueness(repositories)?;
    validate_repositories_urls_and_sources(repositories)?;

    Ok(())
}

// ----------------------------------------------------------------------
// - RepositoryIntern:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct RepositoryIntern {
    repository: Repository,
    pub search_paths: Vec<crate::Uuid>,
}

impl RepositoryIntern {
    const fn new(repository: Repository) -> Self {
        Self {
            repository,
            search_paths: Vec::new(),
        }
    }

    const fn repository(&self) -> &Repository {
        &self.repository
    }
}

fn find_repository_by_uuid<'a, 'b>(
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

    overridden_by: Vec<usize>,
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
                overridden_by: Vec::new(),
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
                if other.repository.repository().is_override() {
                    return Err(Error::Repository(format!("Repository \"{}\" ({}) is overriding another override repository! This is not allowed.", r.name, r.uuid)));
                }
                other.overridden_by.push(idx);
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

    // Sort overrides based on priority:
    for tn in &mut tree_nodes {
        tn.overridden_by.sort_by(|a, b| {
            repositories[*a]
                .repository()
                .cmp(repositories[*b].repository())
        })
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

    let mut node_result = Vec::with_capacity(nodes.len());

    // Handle overrides
    node_result.extend(
        nodes[current_index]
            .overridden_by
            .iter()
            .map(|idx| nodes[*idx].repository.repository().uuid),
    );

    node_result.push(nodes[current_index].repository.repository().uuid); // Add self after all the overrides

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
    for idx in &nodes[current_index].overridden_by {
        result[*idx] = node_result.clone()
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

            b.cmp(a)
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

fn update_repository_search_paths(repositories: &mut [RepositoryIntern]) -> Result<Vec<Uuid>> {
    validate_repositories(repositories)?;

    let (search_paths, global_repository_search_path) =
        calculate_repository_search_paths(repositories)?;

    for (idx, r) in repositories.iter_mut().enumerate() {
        let sp = search_paths[idx].clone();
        assert!(!sp.is_empty());
        r.search_paths = sp;
    }

    assert_eq!(
        global_repository_search_path.is_empty(),
        repositories.is_empty()
    );
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
    /// Create a new `RepositoryDb` populated with all repositories found in `repository_directory`.
    ///
    /// # Errors
    /// Return `Error::Repository` if there are inconsistencies discovered while loading the repositories
    pub fn open(repository_directory: &std::path::Path) -> Result<Self> {
        let mut repositories = backend::read_repositories(repository_directory)?
            .into_iter()
            .map(RepositoryIntern::new)
            .collect::<Vec<_>>();

        let global_repository_search_path = update_repository_search_paths(&mut repositories[..])?;

        tracing::info!(
            "Repository DB with {} repositories created from {:?}.",
            &repositories.len(),
            repository_directory.display(),
        );

        Ok(Self {
            repositories,
            global_repository_search_path,
        })
    }

    /// Resolve some user provided `&str` to an `Repository` `Uuid`.
    #[must_use]
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

    /// Find a `Repository` that will adopt packets from a specific `Packet` source directory.
    #[must_use]
    pub fn repository_for_packet_source_path(&self, input: &std::path::Path) -> Option<Uuid> {
        self.repositories
            .iter()
            .find(|r| {
                let r = r.repository();
                match &r.source {
                    crate::RepositorySource::Local(lr) => {
                        input.starts_with(&lr.sources_base_directory)
                    }
                    crate::RepositorySource::Remote(_) => false,
                }
            })
            .map(|r| r.repository.uuid)
    }

    /// Get the search path for a `Repository`.
    /// A `uuid` of `None` will return the global search path
    #[must_use]
    pub fn search_path(&self, uuid: Option<&Uuid>) -> Vec<Uuid> {
        match uuid {
            None => self.global_repository_search_path.clone(),
            Some(uuid) => find_repository_by_uuid(&self.repositories, uuid)
                .map_or(Vec::new(), |r| r.search_paths.clone()),
        }
    }

    /// Get a `Repository`.
    #[must_use]
    pub fn repository(&self, uuid: &Uuid) -> Option<Repository> {
        find_repository_by_uuid(&self.repositories, uuid).map(|r| r.repository().clone())
    }

    /// Get all repositories
    #[must_use]
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

    /// Sanity check all known repositories
    ///
    /// # Errors
    /// `Error::Repository` might be returned if inconsistencies are detected.
    pub fn fsck(&self) -> Result<bool> {
        let mut repositories = self.repositories.clone();
        update_repository_search_paths(&mut repositories)?;

        Ok(true)
    }
}

impl Default for RepositoryDb {
    #[tracing::instrument(level = "trace")]
    fn default() -> Self {
        tracing::info!("Repository DB with 0 repositories created.",);

        Self {
            repositories: Vec::new(),
            global_repository_search_path: Vec::new(),
        }
    }
}

// ----------------------------------------------------------------------
// - Backend:
// ----------------------------------------------------------------------

mod backend {
    use crate::{Error, Repository, Result};

    pub fn read_repositories(repository_directory: &std::path::Path) -> Result<Vec<Repository>> {
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

    use crate::Repository;

    use super::*;

    #[test]
    fn repository_validation_ok() {
        let mut repositories = [
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

        update_repository_search_paths(&mut repositories).unwrap();
    }

    #[test]
    fn repository_validation_duplicate_name() {
        let mut repositories = [
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

        assert!(update_repository_search_paths(&mut repositories).is_err());
    }

    #[test]
    fn repository_validation_duplicate_uuid() {
        let uuid = Uuid::new_v4();
        let mut repositories = [
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

        assert!(update_repository_search_paths(&mut repositories).is_err());
    }

    #[test]
    fn repository_validation_dependency_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let mut repositories = [
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

        assert!(update_repository_search_paths(&mut repositories).is_err());
    }

    #[test]
    fn repository_validation_override_loop() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let mut repositories = [
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

        assert!(update_repository_search_paths(&mut repositories).is_err());
    }

    #[test]
    fn repository_validation_unknown_dependency() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let mut repositories = [RepositoryIntern::new(Repository {
            name: Name::try_from("base_repo").expect("Name was valid!"),
            uuid: uuid1,
            priority: 100,
            source: crate::RepositorySource::Local(crate::LocalRepository {
                sources_base_directory: std::path::PathBuf::from("file:///tmp/sources/base"),
                export_directory: None,
            }),
            relation: crate::RepositoryRelation::Dependency(vec![uuid2]),
        })];

        assert!(update_repository_search_paths(&mut repositories).is_err());
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

    fn create_override_repo(
        name: &str,
        uuid: &Uuid,
        overrides: Uuid,
        priority: u32,
    ) -> RepositoryIntern {
        RepositoryIntern::new(Repository {
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

        let mut repositories = [
            create_dependent_repo("r3", &uuid_3, vec![uuid_2]),
            create_override_repo("r1o0", &uuid_1o0, uuid_1, 10000),
            create_override_repo("r2o1", &uuid_2o1, uuid_2, 2000),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0]),
            create_override_repo("r2o0", &uuid_2o0, uuid_2, 15000),
            create_dependent_repo("r0", &uuid_0, vec![]),
            create_dependent_repo("r2", &uuid_2, vec![uuid_1]),
        ];

        let global_search_path = update_repository_search_paths(&mut repositories)
            .expect("Input was supposed to be correct");

        assert_eq!(
            global_search_path,
            vec![uuid_3, uuid_2o0, uuid_2o1, uuid_2, uuid_1o0, uuid_1, uuid_0]
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

        let mut repositories = [
            create_dependent_repo("r0", &uuid_0, vec![]),
            create_dependent_repo("r1", &uuid_1, vec![uuid_0]),
            create_dependent_repo("r2l0", &uuid_2left0, vec![uuid_1]),
            create_dependent_repo("r2l1", &uuid_2left1, vec![uuid_2left0]),
            create_dependent_repo("r2r0", &uuid_2right0, vec![uuid_1]),
            create_override_repo("r2r0o0", &uuid_2right0o0, uuid_2right0, 99),
            create_dependent_repo("r3", &uuid_3, vec![uuid_2left1, uuid_2right0]),
        ];

        let global_search_path = update_repository_search_paths(&mut repositories)
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
}
