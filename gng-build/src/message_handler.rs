// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle messages from `gng-build-agent`

use gng_build_shared::SourcePacket;
use sha3::{Digest, Sha3_256};

// - Helper:
// ----------------------------------------------------------------------

fn hash_str(input: &str) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(input.as_bytes());
    let mut v = Vec::with_capacity(Sha3_256::output_size());
    v.extend_from_slice(&hasher.finalize());

    v
}

// ----------------------------------------------------------------------
// - Message Handler:
// ----------------------------------------------------------------------

/// An object used to handle messages from the `gng-build-agent`
pub trait MessageHandler {
    /// Verify state before `gng-build-agent` is started
    ///
    /// # Errors
    /// Generic Error
    fn prepare(&mut self, mode: &crate::Mode) -> eyre::Result<()>;

    /// Handle one message from `gng-build-agent`
    ///
    /// # Errors
    /// Generic Error
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool>;

    /// Verify state after `gng-build-agent` has quit successfully
    ///
    /// # Errors
    /// Generic Error
    fn verify(&mut self, mode: &crate::Mode) -> eyre::Result<()>;
}

// ----------------------------------------------------------------------
// - ImmutableSourceDataHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
#[derive(Debug)]
pub struct ImmutableSourceDataHandler {
    hash: Option<Vec<u8>>,
    first_message: bool,
}

impl Default for ImmutableSourceDataHandler {
    fn default() -> Self {
        Self {
            hash: None,
            first_message: true,
        }
    }
}

impl MessageHandler for ImmutableSourceDataHandler {
    #[tracing::instrument(level = "trace")]
    fn prepare(&mut self, mode: &crate::Mode) -> eyre::Result<()> {
        self.first_message = true;
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool> {
        if message_type != &gng_build_shared::MessageType::DATA {
            self.first_message = false;
            return Ok(false);
        }

        if !self.first_message {
            tracing::error!("The build agent did not send a DATA message first!");
            panic!("gng-build-agent did not react as expected!");
        }

        self.first_message = false;

        let v = hash_str(message);

        match self.hash.as_ref() {
            None => {
                self.hash = Some(v);
                Ok(false)
            }
            Some(vg) if *vg == v => Ok(false),
            Some(_) => {
                tracing::error!("Source data changed, aborting!");
                panic!("gng-build-agent did not react as expected!");
            }
        }
    }

    #[tracing::instrument(level = "trace")]
    fn verify(&mut self, mode: &crate::Mode) -> eyre::Result<()> {
        if self.first_message {
            tracing::error!("The build agent did not send any message!");
            panic!("gng-build-agent did not react as expected!");
        }

        if self.hash.is_none() {
            tracing::error!("No source data received during QUERY mode.");
            panic!("gng-build-agent did not react as expected!");
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - PackageHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct PacketHandler {
    source_packet: Option<SourcePacket>,
}

impl Default for PacketHandler {
    fn default() -> Self {
        Self {
            source_packet: None,
        }
    }
}

impl MessageHandler for PacketHandler {
    fn prepare(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        Ok(())
    }

    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool> {
        if *mode != crate::Mode::PACKAGE && message_type != &gng_build_shared::MessageType::DATA {
            return Ok(false);
        }

        self.source_packet = Some(serde_json::from_str(message).map_err(|e| eyre::eyre!(e))?);

        Ok(false)
    }

    fn verify(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        assert!(self.source_packet.is_some());

        todo!();
    }
}
