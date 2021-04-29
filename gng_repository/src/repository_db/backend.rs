// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A backend database for a `Repository`

use std::convert::TryFrom;

use super::definitions::RepositoryInternal;
use crate::Repository;

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

const MAGIC: &str = "\"1e925d91-3294-4676-add6-917376d89d58\"";
const MAGIC_KEY: &str = "magic";

const VERSION: u32 = 1;
const VERSION_KEY: &str = "schema_version";

const REPOSITORIES_TREE: &str = "repositories";
const PACKETS_TREE: &str = "packets";
const CONTENTS_TREE: &str = "contents";

const PACKET_REPO_TREE_PREFIX: &str = "packets_";

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

#[tracing::instrument(level = "trace", skip(db))]
pub fn open_db(db: &sled::Db) -> crate::Result<()> {
    if db.is_empty() {
        setup(db)?
    };

    match db.get(MAGIC_KEY)? {
        Some(v) if v == MAGIC => validate(db),
        _ => Err(crate::Error::WrongMagic),
    }
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn schema_version(db: &sled::Db) -> crate::Result<u32> {
    let version = db.get(VERSION_KEY)?.expect("Version must be set!");
    serde_json::from_slice(&version[..]).map_err(|_| crate::Error::WrongSchema)
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn setup(db: &sled::Db) -> crate::Result<()> {
    db.insert(
        VERSION_KEY,
        &serde_json::to_vec(&VERSION).map_err(|_| crate::Error::WrongSchema)?[..],
    )?;

    let _repository_tree = db.open_tree(REPOSITORIES_TREE)?;
    let _packages_tree = db.open_tree(PACKETS_TREE)?;
    let _contents_tree = db.open_tree(CONTENTS_TREE)?;

    db.insert(MAGIC_KEY, MAGIC)?;

    db.flush()?;
    Ok(())
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn validate(db: &sled::Db) -> crate::Result<()> {
    match schema_version(db)? {
        v if v < VERSION => Err(crate::Error::WrongSchema),
        v if v == VERSION => Ok(()),
        _ => Err(crate::Error::WrongSchema),
    }
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn read_repositories(db: &sled::Db) -> crate::Result<Vec<crate::Repository>> {
    let tree = db.open_tree(REPOSITORIES_TREE)?;

    let (data, id_map) = {
        let mut data = Vec::with_capacity(tree.len());
        let mut id_map = std::collections::HashMap::with_capacity(tree.len());

        for tree_result in tree.iter() {
            match tree_result {
                Err(e) => return Err(e.into()),
                Ok((k, v)) => {
                    let name: gng_shared::Name =
                        serde_json::from_slice(&k[..]).map_err(|_| crate::Error::WrongSchema)?;
                    let rid: RepositoryInternal =
                        serde_json::from_reader(&v[..]).map_err(|_| crate::Error::WrongSchema)?;

                    id_map.insert(rid.uuid, name.clone());

                    data.push((name, rid));
                }
            }
        }
        (data, id_map)
    };

    let mut result = data
        .iter()
        .map(|(n, d)| -> crate::Result<crate::Repository> {
            let dependencies = gng_shared::Names::from(
                d.dependencies
                    .iter()
                    .map(|u| {
                        let name = id_map.get(u).ok_or_else(|| {
                            crate::Error::Repository(format!(
                                "Unknown repository with Uuid \"{}\".",
                                &u
                            ))
                        })?;
                        Ok(name.clone())
                    })
                    .collect::<crate::Result<Vec<gng_shared::Name>>>()?
                    .as_ref(),
            );

            Ok(crate::Repository {
                name: n.clone(),
                uuid: d.uuid,
                priority: d.priority,
                pull_url: d.pull_url.clone(),
                packet_base_url: d.packet_base_url.clone(),
                sources_base_directory: d.sources_base_directory.clone(),
                dependencies,
                tags: gng_shared::Names::try_from(&d.tags[..])
                    .map_err(|_| crate::Error::WrongSchema)?,
            })
        })
        .collect::<crate::Result<Vec<crate::Repository>>>()?;

    result.sort();

    Ok(result)
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn write_repositories(db: &sled::Db, repositories: &[Repository]) -> crate::Result<()> {
    let tree = db.open_tree(REPOSITORIES_TREE)?;

    let name_map: std::collections::HashMap<gng_shared::Name, crate::Uuid> = repositories
        .iter()
        .map(|r| (r.name.clone(), r.uuid))
        .collect();

    let batch = {
        let mut batch = sled::Batch::default();

        for r in repositories {
            batch.insert(
                serde_json::to_vec(&r.name).expect("names must be convertible to JSON"),
                serde_json::to_vec(&RepositoryInternal {
                    uuid: r.uuid,
                    priority: r.priority,
                    pull_url: r.pull_url.clone(),
                    packet_base_url: r.packet_base_url.clone(),
                    sources_base_directory: r.sources_base_directory.clone(),
                    dependencies: r
                        .dependencies
                        .into_iter()
                        .map(|n| {
                            name_map.get(n).copied().ok_or_else(|| {
                                crate::Error::Repository(format!(
                                    "Unknown repository with Uuid \"{}\".",
                                    &n
                                ))
                            })
                        })
                        .collect::<crate::Result<Vec<crate::Uuid>>>()?,
                    tags: r
                        .tags
                        .into_iter()
                        .map(gng_shared::Name::to_string)
                        .collect(),
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

#[tracing::instrument(level = "trace", skip(db))]
pub fn remove_repository(db: &sled::Db, name: &str, uuid: &crate::Uuid) -> crate::Result<()> {
    let tree = db.open_tree(REPOSITORIES_TREE)?;

    tree.remove(&serde_json::to_vec(&name).expect("Names can be serialized!")[..])?;
    tree.flush()?;

    let repository_packets_tree = format!("{}{}", PACKET_REPO_TREE_PREFIX, uuid);
    db.drop_tree(&repository_packets_tree[..])
        .map(|_| ())
        .map_err(crate::Error::Backend)
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn dump_metadata(db: &sled::Db) -> crate::Result<()> {
    println!("Metadata:");
    for data in db.iter() {
        match data {
            Err(e) => println!("Error: {}", e),
            Ok((k, v)) => {
                let key =
                    std::string::String::from_utf8((&k[..]).to_vec()).unwrap_or(format!("{:?}", k));
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
    for n in &db.tree_names() {
        let n = std::string::String::from_utf8((&n[..]).to_vec()).unwrap_or(format!("{:?}", n));
        if n.starts_with("__") {
            continue;
        }
        println!("    \"{}\"", &n)
    }
    Ok(())
}
