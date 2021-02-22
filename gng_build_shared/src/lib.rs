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
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

/// Shared constants:
pub mod constants {
    /// Container related constants
    pub mod container {
        use std::path::PathBuf;

        lazy_static::lazy_static! {
            /// The directory inside the build container with all the gng-related files and folders
            pub static ref GNG_DIR: PathBuf = PathBuf::from("/gng");
            /// The `gng-build-agent` binary inside the container
            pub static ref GNG_BUILD_AGENT_EXECUTABLE: PathBuf = GNG_DIR.join("build-agent");
            /// The `src` folder inside the build container
            pub static ref GNG_WORK_DIR: PathBuf = GNG_DIR.join("work");
            /// The `inst` folder inside the build container
            pub static ref GNG_INST_DIR: PathBuf = GNG_DIR.join("inst");
            /// The `lua` folder inside the build container
            pub static ref GNG_LUA_DIR: PathBuf = GNG_DIR.join("lua");
        }
    }

    /// Environment variable names:
    pub mod environment {
        /// `GNG_BUILD_AGENT` environment variable name
        pub const GNG_BUILD_AGENT: &str = "GNG_BUILD_AGENT";
        /// `GNG_WORK_DIR` environment variable name
        pub const GNG_WORK_DIR: &str = "GNG_WORK_DIR";
        /// `GNG_INST_DIR` environment variable name
        pub const GNG_INST_DIR: &str = "GNG_INST_DIR";
        /// `GNG_LUA_DIR` environment variable name
        pub const GNG_LUA_DIR: &str = "GNG_LUA_DIR";

        /// `GNG_AGENT_MESSAGE_PREFIX` environment variable name
        pub const GNG_AGENT_MESSAGE_PREFIX: &str = "GNG_AGENT_MESSAGE_PREFIX";
        /// `GNG_AGENT_OUTPUT_PREFIX` environment variable name
        pub const GNG_AGENT_OUTPUT_PREFIX: &str = "GNG_AGENT_OUTPUT_PREFIX";
    }
}

/// Types of messages going from `gng-build-agent` back to `gng-build`
#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    /// Source packet data
    DATA,
    /// Test data
    TEST,
}

impl std::convert::TryFrom<String> for MessageType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if &value == "DATA" {
            Ok(Self::DATA)
        } else if &value == "TEST" {
            Ok(Self::TEST)
        } else {
            Err(format!("Failed to convert {} to MessageType", value))
        }
    }
}

impl std::convert::From<&MessageType> for String {
    fn from(mt: &MessageType) -> Self {
        match mt {
            MessageType::DATA => Self::from("DATA"),
            MessageType::TEST => Self::from("TEST"),
        }
    }
}

/// The build script to use
pub const BUILD_SCRIPT: &str = "build.lua";

// ----------------------------------------------------------------------
// - Sub-Modules:
// ----------------------------------------------------------------------

mod source_packet;

pub use source_packet::{PacketDefinition, Source, SourcePacket};
