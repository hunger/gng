// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A backend database for a `RepositoryDb`

use super::definitions::{HashedPackets, RepositoryIntern};
use crate::{Error, Repository, Result};

use gng_shared::Name;

use std::convert::TryFrom;

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

const META_FILE: &str = "meta.json";
const REPOSITORY_DIR: &str = "repos";
const PACKETS_FILE: &str = "packets.json";

const MAGIC: &str = "1e925d91-3294-4676-add6-917376d89d58";
const SCHEMA_VERSION: u32 = 1;

// ----------------------------------------------------------------------
// - MetaData:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MetaData {
    magic: crate::Uuid,
    schema: u32,
}

impl MetaData {
    pub fn has_valid_magic(&self) -> bool {
        self.magic == crate::Uuid::parse_str(MAGIC).expect("UUID constant needs to be valid!")
    }

    pub const fn latest_schema() -> u32 {
        SCHEMA_VERSION
    }

    pub const fn current_schema(&self) -> u32 {
        self.schema
    }
}

impl Default for MetaData {
    fn default() -> Self {
        Self {
            magic: crate::Uuid::parse_str(MAGIC).expect("UUID constant needs to be valid!"),
            schema: SCHEMA_VERSION,
        }
    }
} // Default for MetaData

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn get_db_schema_version(db_directory: &std::path::Path) -> crate::Result<u32> {
    if !db_directory.exists() {
        return Err(crate::Error::Db(format!(
            "\"{}\" does not exist.",
            db_directory.to_string_lossy()
        )));
    }
    if !db_directory.is_dir() {
        return Err(crate::Error::Db(format!(
            "\"{}\" is not a directory.",
            db_directory.to_string_lossy()
        )));
    }

    let meta_file = db_directory.join(META_FILE);
    if !meta_file.exists() && std::fs::read_dir(db_directory)?.count() != 0 {
        return Err(crate::Error::Db(format!(
            "\"{}\" is not empty and has no meta.json file.",
            db_directory.to_string_lossy()
        )));
    }
    if meta_file.exists() && !meta_file.is_file() {
        return Err(crate::Error::Db(format!(
            "\"{}\" is not a file.",
            meta_file.to_string_lossy()
        )));
    }
    if meta_file.exists() && meta_file.metadata()?.len() > 1024 {
        return Err(crate::Error::Db(format!(
            "\"{}\" is too big.",
            meta_file.to_string_lossy()
        )));
    }

    let meta_data = if meta_file.exists() {
        let mf = std::fs::OpenOptions::new().read(true).open(&meta_file)?;
        serde_json::from_reader(&mf)
            .map_err(|_| crate::Error::Db("Failed to parse meta file.".to_string()))?
    } else {
        MetaData::default()
    };

    if !meta_data.has_valid_magic() {
        return Err(crate::Error::Db("Magic was not valid.".to_string()));
    }

    Ok(meta_data.current_schema())
}

fn read_repositories(db_directory: &std::path::Path) -> Result<Vec<Repository>> {
    let repos_directories = db_directory.join(REPOSITORY_DIR);
    let mut result = Vec::new();

    if let Ok(repository_files) = std::fs::read_dir(repos_directories) {
        for entry in repository_files {
            match entry {
                Ok(entry) => {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name
                        .rsplit('.')
                        .next()
                        .map(|ext| ext.eq_ignore_ascii_case("json"))
                        != Some(true)
                    {
                        tracing::warn!("\"{}\" si not a .json file, skipping.", file_name);
                        continue; // ignore non-json files!
                    }
                    let end = file_name.len() - 5;
                    Name::try_from(&file_name[..end]).map_err(|_| {
                        Error::Repository(format!("Invalid repository name in \"{}\".", &file_name))
                    })?;

                    let fd = std::fs::OpenOptions::new()
                        .read(true)
                        .open(entry.path())
                        .map_err(|e| {
                            Error::Repository(format!(
                                "Failed to open repository data \"{}\": {}.",
                                file_name, e
                            ))
                        })?;
                    result.push(serde_json::from_reader(fd).map_err(|e| {
                        Error::Repository(format!("Failed to parse \"{}\": {}.", file_name, e))
                    })?);
                }
                Err(e) => tracing::warn!("Could not retrieve directory data: {}", e),
            }
        }
    }

    Ok(result)
}

#[tracing::instrument(level = "trace")]
pub fn init_db(db_directory: &std::path::Path) -> crate::Result<()> {
    tracing::debug!("Initializing DB.");
    std::fs::create_dir_all(db_directory).map_err(|e| {
        Error::Db(format!(
            "Can not initialize DB at \"{}\": {}.",
            db_directory.to_string_lossy(),
            e
        ))
    })?;

    let meta_file = db_directory.join(META_FILE);
    if meta_file.exists() && !meta_file.is_file() {
        return Err(Error::Db(format!(
            "Metadata exists, but is not a file in \"{}\".",
            db_directory.to_string_lossy(),
        )));
    }

    if !meta_file.exists() {
        tracing::debug!("Creating Meta data file.");

        let meta_fd = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(meta_file)
            .map_err(|e| {
                Error::Db(format!(
                    "Can not open meta info of DB at \"{}\": {}.",
                    db_directory.to_string_lossy(),
                    e
                ))
            })?;

        serde_json::to_writer(meta_fd, &MetaData::default()).map_err(|e| {
            Error::Db(format!(
                "Can not write meta info of DB at \"{}\": {}.",
                db_directory.to_string_lossy(),
                e
            ))
        })?;
    }

    tracing::trace!("Creating DB sub folders");
    let repos_directory = db_directory.join(REPOSITORY_DIR);
    if !repos_directory.is_dir() {
        tracing::debug!("Creating DB sub folder for repository information.");
        std::fs::create_dir(&repos_directory).map_err(|e| {
            Error::Db(format!(
                "Can not repository directory for DB at \"{}\": {}.",
                db_directory.to_string_lossy(),
                e
            ))
        })?;
    }

    Ok(())
}

#[tracing::instrument(level = "trace")]
pub fn read_db(
    db_directory: &std::path::Path,
) -> crate::Result<(Vec<RepositoryIntern>, HashedPackets)> {
    if get_db_schema_version(db_directory)? != MetaData::latest_schema() {
        return Err(crate::Error::Db("Unsupported schema version.".to_string()));
    }

    tracing::debug!("Reading repositories from DB.");
    let repositories = read_repositories(db_directory)?;

    Ok((
        repositories
            .into_iter()
            .map(RepositoryIntern::new)
            .collect(),
        HashedPackets::new(),
    ))
}

fn repository_file_name(
    db_directory: &std::path::Path,
    repository_name: &Name,
) -> std::path::PathBuf {
    db_directory
        .join(REPOSITORY_DIR)
        .join(format!("{}.json", repository_name))
}

#[tracing::instrument(level = "trace")]
pub fn persist_repository(
    db_directory: &std::path::Path,
    repository: &Repository,
) -> crate::Result<()> {
    let repository_name = &repository.name;
    tracing::debug!("Persisting repository \"{}\".", &repository_name);

    let repository_file = repository_file_name(db_directory, repository_name);
    let rf = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&repository_file)
        .map_err(|e| {
            Error::Db(format!(
                "Failed to persist repository \"{}\": {}.",
                repository_name, e,
            ))
        })?;

    serde_json::to_writer_pretty(&rf, repository).map_err(|e| {
        Error::Db(format!(
            "Failed to persist repository \"{}\": {}",
            repository_name, e
        ))
    })
}

#[tracing::instrument(level = "trace")]
pub fn remove_repository(
    db_directory: &std::path::Path,
    repository_name: &gng_shared::Name,
) -> Result<()> {
    tracing::debug!("Removing repository \"{}\".", &repository_name);
    todo!()
}

/*
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

    let tree = db.open_tree(REPOSITORIES_TREE)?;
    tree.apply_batch(batch)?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn remove_repository(db: &sled::Db, name: &str, uuid: &crate::Uuid) -> crate::Result<()> {
    let repo_tree = db.open_tree(REPOSITORIES_TREE)?;
    let repo_packets_tree =  db.open_tree(REPOSITORY_PACKETS_TREE)?;

    let prefix = format!("{}/", uuid);

    let batch = {
        let mut tmp = sled::Batch::default();

        for e in repo_packets_tree.scan_prefix(prefix.as_bytes()) {
            tmp.remove(e?.0);
        }
        tmp
    };

    (&repo_tree, &repo_packets_tree).transaction(|(tx_repo, tx_packets)| {
        tx_repo.remove(&serde_json::to_vec(&name).expect("Names can be serialized!")[..])?;
        tx_packets.apply_batch(&batch)?;
        Ok(())
    }).map_err(|e| match e {
        sled::transaction::TransactionError::Abort(c) => c,
        sled::transaction::TransactionError::Storage(s) => crate::Error::Backend(s),
    })
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

#[tracing::instrument(level = "trace", skip(db))]
pub fn store_hashed_packet(db: &sled::Db, hash: &str, data: &[u8]) -> crate::Result<()> {
    let tree = db.open_tree(PACKET_TREE)?;

    if let Some(old_data) = tree.insert(hash, data)? {
        if old_data.as_ref() == data {
            tracing::trace!("Packet with hash \"{}\" stored, replacing same data.", hash);
        } else {
            return Err(crate::Error::Packet(format!(
                "Changed contents of packet with hash \"{}\".",
                hash
            )));
        }
    } else {
        tracing::trace!("Packet with hash \"{}\" stored in empty slot.", hash);
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn flush(db: &sled::Db) -> crate::Result<()> {
    db.flush().map_err(crate::Error::Backend)?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn repository_contains_name(
    db: &sled::Db,
    repository: &crate::Uuid,
    name: &gng_shared::Name,
) -> Option<crate::Uuid> {
    let repo_tree = format!("{}{}", PACKET_REPO_TREE_PREFIX, repository);

    match db.open_tree(repo_tree) {
        Ok(t) => t
            .contains_key(name.as_bytes())
            .unwrap_or(false)
            .then(|| *repository),
        Err(_) => None,
    }
}

#[tracing::instrument(level = "trace", skip(db))]
pub fn repository_group_contains_name(
    db: &sled::Db,
    group: &[crate::Uuid],
    name: &gng_shared::Name,
) -> Option<crate::Uuid> {
    for r in group {
        let result = repository_contains_name(db, r, name);
        if result.is_some() {
            return result;
        }
    }
    None
}
*/
