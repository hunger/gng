// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A directory based packet DB

use gng_core::{Name, Names};

use eyre::{eyre, WrapErr};

use std::io::{BufRead, Write};

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn relative_file_path(
    base_url: &url::Url,
    file_path: &std::path::Path,
) -> eyre::Result<(std::path::PathBuf, std::path::PathBuf)> {
    let repository_file = base_url.to_file_path().map_err(|_| {
        eyre!(
            "Failed to turn base URL \"{}\" into a file path.",
            base_url.as_str()
        )
    })?;
    if !repository_file.is_file() {
        return Err(eyre!(
            "Base URL \"{}\" does not point to a repository file.",
            base_url.as_str()
        ));
    }

    let base_dir = if let Some(base_dir) = repository_file.parent() {
        base_dir
    } else {
        return Err(eyre!(
            "Parent of base URL \"{}\" does not exist.",
            base_url.as_str()
        ));
    };

    if !base_dir.is_dir() {
        return Err(eyre!(
            "Base URL \"{}\" does not point to a directory.",
            base_url.as_str()
        ));
    }
    let file_path = if file_path.is_relative() {
        file_path
    } else {
        file_path.strip_prefix(&base_dir).wrap_err(eyre!(
            "\"{}\" must be inside \"{}\".",
            file_path.to_string_lossy(),
            &base_dir.to_string_lossy(),
        ))?
    };
    let abs_file_path = base_dir.join(file_path);
    if !abs_file_path.is_file() {
        return Err(eyre!(
            "\"{}\" is not a file.",
            &abs_file_path.to_string_lossy()
        ));
    }

    Ok((file_path.to_path_buf(), abs_file_path))
}

// ----------------------------------------------------------------------
// - Entry:
// ----------------------------------------------------------------------

#[derive(Clone, Debug, Eq, serde::Deserialize, serde::Serialize)]
struct Entry {
    // FIXME: Add a file hash!
    #[serde(rename = "packet")]
    packet_data: gng_packet_io::BinaryPacketDefinition,
    #[serde(rename = "file")]
    file_path: std::path::PathBuf, // relative to the DB file!
}

impl Entry {
    fn from_packet_file(
        repository_url: &url::Url,
        file_path: &std::path::Path,
    ) -> eyre::Result<Self> {
        let (rel_file_path, abs_file_path) = relative_file_path(repository_url, file_path)?;
        let mut packet_reader = gng_packet_io::PacketReader::new(&abs_file_path);
        let packet_data = packet_reader.metadata().wrap_err(eyre!(
            "Failed to read packet data from \"{}\".",
            &abs_file_path.to_string_lossy(),
        ))?;

        Ok(Self {
            packet_data,
            file_path: rel_file_path,
        })
    }

    fn from_json(json: &str) -> eyre::Result<Self> {
        let me: Self = serde_json::from_str(json).wrap_err("Failed to read json data")?;
        if me.file_path.is_relative() {
            Ok(me)
        } else {
            Err(eyre!("File path read from json is not relative"))
        }
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.packet_data.cmp(&other.packet_data)
    }
}

// ----------------------------------------------------------------------
// - RepositoryTransaction:
// ----------------------------------------------------------------------

/// A transaction to update a `Repository`
pub struct Update {
    repository_url: url::Url,
    to_apply: Vec<Entry>,
    to_remove: Names,
    do_clear: bool,
}

impl Update {
    /// Remove a packet from the Repository
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn remove(&mut self, name: Name) {
        self.to_remove.insert(name);
    }

    /// Clear all data from the repository
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn clear(&mut self) {
        self.do_clear = true;
        self.to_apply = Vec::new();
        self.to_remove = Names::default();
    }

    fn add_entry(&mut self, entry: Entry) {
        self.remove(entry.packet_data.name.clone());
        self.to_apply.push(entry);
    }

    /// Add a packet to the transaction
    ///
    /// # Errors
    ///
    /// Errors out when the packet is not valid.
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn add_packet_file(&mut self, packet_file_path: &std::path::Path) -> eyre::Result<()> {
        tracing::debug!(
            "Adding packet file \"{}\" to repository \"{}\"",
            packet_file_path.to_string_lossy(),
            self.repository_url.as_str(),
        );

        self.add_entry(Entry::from_packet_file(
            &self.repository_url,
            packet_file_path,
        )?);
        Ok(())
    }

    /// Read Entries from a file.
    ///
    /// # Errors
    /// Errors out if reading fails
    #[tracing::instrument(level = "debug", skip(self))]

    pub fn read_entries_file(&mut self, entries_file: &std::path::Path) -> eyre::Result<()> {
        tracing::debug!(
            "Reading repository data from \"{}\"",
            entries_file.to_string_lossy()
        );

        let reader = std::fs::File::open(entries_file).wrap_err(eyre!(
            "Failed to open repository data in \"{}\".",
            entries_file.to_string_lossy()
        ))?;
        let reader = std::io::BufReader::new(reader);

        for l in reader.lines() {
            let l = l.wrap_err(eyre!(
                "Failed to read data entry from \"{}\".",
                entries_file.to_string_lossy()
            ))?;
            self.add_entry(Entry::from_json(&l).wrap_err(eyre!(
                "Parsing data in repository file \"{}\" failed.",
                entries_file.to_string_lossy()
            ))?);
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - Repository:
// ----------------------------------------------------------------------

/// A packet DB that just gets packets from a given directory
pub struct Repository {
    base_url: url::Url,

    packets: Vec<Entry>,
}

impl Repository {
    /// Constructor
    #[must_use]
    #[tracing::instrument(level = "debug")]
    pub fn new(base_url: &url::Url) -> Self {
        Self {
            base_url: base_url.clone(),
            packets: Vec::new(),
        }
    }

    /// Open a local directory containing a repository file.
    ///
    /// # Errors
    ///
    /// Fail is something goes wrong.
    #[tracing::instrument(level = "debug")]
    pub fn from_local_directory(
        repository_directory: &std::path::Path,
        create_if_missing: bool,
    ) -> eyre::Result<Self> {
        let entries_file = repository_directory.join("repository.json");

        let mut repo = Self {
            base_url: url::Url::from_file_path(&entries_file).map_err(|_| {
                eyre!(
                    "Failed fo convert entries file \"{}\" to URL",
                    entries_file.to_string_lossy()
                )
            })?,
            packets: Vec::new(),
        };

        // Make sire the file exists:
        if create_if_missing && !entries_file.exists() {
            tracing::debug!(
                "Creating empty repository data file \"{}\"",
                entries_file.to_string_lossy()
            );
            repo.save(&entries_file)?;
        }

        let mut trans = repo.create_transaction();
        trans.read_entries_file(&entries_file)?;
        repo.apply(trans).map(|_| repo)
    }

    /// Create a new transaction.
    #[must_use]
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn create_transaction(&mut self) -> Update {
        Update {
            repository_url: self.base_url.clone(),
            to_apply: Vec::new(),
            to_remove: Names::default(),
            do_clear: false,
        }
    }

    /// Apply a transaction to this repository.
    ///
    /// # Errors
    /// Errors out when the update does not match with this repository.
    #[tracing::instrument(level = "debug", skip(self, update))]
    pub fn apply(&mut self, update: Update) -> eyre::Result<()> {
        if update.repository_url != self.base_url {
            return Err(eyre!("Updated has wrong repository URL!"));
        }

        let mut new_packets = if update.do_clear {
            Vec::new()
        } else {
            self.packets
                .iter()
                .filter(|f| !update.to_remove.contains(&f.packet_data.name))
                .cloned()
                .collect::<Vec<_>>()
        };

        new_packets.extend_from_slice(&update.to_apply[..]);

        new_packets.sort();

        self.packets = new_packets;

        Ok(())
    }

    /// Query a packet/facet name combination
    #[tracing::instrument(level = "trace", skip(self))]
    #[must_use]
    pub fn query(
        &self,
        packet: &Name,
        facet: &Option<Name>,
    ) -> Option<(gng_packet_io::BinaryPacketDefinition, std::path::PathBuf)> {
        None
    }

    /// Save the entries to a file.
    ///
    /// # Errors
    /// Errors out if the file can not get written.
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn save(&self, entries_file: &std::path::Path) -> eyre::Result<()> {
        tracing::debug!(
            "Saving repository data into \"{}\"",
            entries_file.to_string_lossy()
        );

        let writer = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(entries_file)
            .wrap_err(eyre!(
                "Failed to open repository file \"{}\" for writing",
                entries_file.to_string_lossy()
            ))?;
        let mut writer = std::io::BufWriter::new(writer);

        for e in &self.packets {
            let mut data = serde_json::to_vec(&e).wrap_err(eyre!(
                "Failed to serialize repository data for file \"{}\"",
                entries_file.to_string_lossy(),
            ))?;
            data.push(b'\n');

            writer.write_all(&data).wrap_err(eyre!(
                "Failed to write data into \"{}\"",
                entries_file.to_string_lossy()
            ))?;
        }
        Ok(())
    }

    /// Save the entries to a file in a default location.
    ///
    /// # Errors
    /// Errors out if the file can not get written.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn save_local_directory(&self) -> eyre::Result<()> {
        let entries_file = self.base_url.to_file_path().map_err(|_| {
            eyre!(
                "Failed to turn base URL \"{}\" into a file path.",
                self.base_url.as_str()
            )
        })?;

        self.save(&entries_file)
    }
}
