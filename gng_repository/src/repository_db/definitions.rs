// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Definition of data types used by the `Repository`

// ----------------------------------------------------------------------
// - RepositoryInternal:
// ----------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, serde::Serialize)]
/// Data on a repository stored in a `Repository`.
pub struct RepositoryInternal {
    /// The repository UUID
    pub uuid: crate::Uuid,
    /// The priority of this `RepositoryData`
    pub priority: u32,
    /// The url to pull updates from
    pub pull_url: Option<String>,
    /// The base URL to download `Packet`s from
    pub packet_base_url: String,
    /// The base directory holding the source packages for this repository.
    pub sources_base_directory: Option<std::path::PathBuf>,
    /// `RepositoryData` this one depends on
    pub dependencies: Vec<crate::Uuid>,
    /// Tags on a `Repository`.
    pub tags: Vec<String>,
}
