// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Functionality shared by `gng-build` and `gng-build-agent`

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

/// Container related constants
pub mod cnt {
    use std::path::PathBuf;

    lazy_static::lazy_static! {
        /// The directory inside the build container with all the gng-related files and folders
        pub static ref GNG_DIR: PathBuf = PathBuf::from("/gng");
        /// The `gng-build-agent` binary inside the container
        pub static ref GNG_BUILD_AGENT_EXECUTABLE: PathBuf = GNG_DIR.join("build-agent");
        /// The `pkgsrc` folder inside the build container
        pub static ref GNG_PKGSRC_DIR: PathBuf = GNG_DIR.join("pkgsrc");
        /// The `src` folder inside the build container
        pub static ref GNG_SRC_DIR: PathBuf = GNG_DIR.join("src");
        /// The `inst` folder inside the build container
        pub static ref GNG_INST_DIR: PathBuf = GNG_DIR.join("inst");
        /// The `pkg` folder inside the build container
        pub static ref GNG_PKG_DIR: PathBuf = GNG_DIR.join("pkg");
    }
}

/// Environment variable names:
pub mod env {
    /// GNG_BUILD_AGENT:
    pub const GNG_BUILD_AGENT: &str = "GNG_BUILD_AGENT";
    /// GNG_PKGSRC_DIR:
    pub const GNG_PKGSRC_DIR: &str = "GNG_PKGSRC_DIR";
    /// GNG_PKGSRC_DIR:
    pub const GNG_SRC_DIR: &str = "GNG_SRC_DIR";
    /// GNG_PKGSRC_DIR:
    pub const GNG_INST_DIR: &str = "GNG_INST_DIR";
    /// GNG_PKGSRC_DIR:
    pub const GNG_PKG_DIR: &str = "GNG_PKG_DIR";

    /// GNG_AGENT_MESSAGE_PREFIX:
    pub const GNG_AGENT_MESSAGE_PREFIX: &str = "GNG_AGENT_MESSAGE_PREFIX";
    /// A prefix to mark up normal messages as originating in `gng-build-agent`
    pub const GNG_AGENT_OUTPUT_PREFIX: &str = "GNG_AGENT_OUTPUT_PREFIX";
}

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------
