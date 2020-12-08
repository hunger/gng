// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle messages from `gng-build-agent`

use gng_build_shared::SourcePacket;

use eyre::Result;
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
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()>;

    /// Handle one message from `gng-build-agent`
    ///
    /// # Errors
    /// Generic Error
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool>;

    /// Verify state after `gng-build-agent` has quit successfully
    ///
    /// # Errors
    /// Generic Error
    fn verify(&mut self, mode: &crate::Mode) -> Result<()>;
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
    fn prepare(&mut self, mode: &crate::Mode) -> Result<()> {
        self.first_message = true;
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> Result<bool> {
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
    fn verify(&mut self, mode: &crate::Mode) -> Result<()> {
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
    fn prepare(&mut self, _mode: &crate::Mode) -> Result<()> {
        Ok(())
    }

    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        _message: &str,
    ) -> Result<bool> {
        if *mode != crate::Mode::QUERY && message_type != &gng_build_shared::MessageType::DATA {
            return Ok(false);
        }

        // let source_packet = serde_json::from_str(message).map_err(|e| eyre!(e))?;

        Ok(false)
    }

    fn verify(&mut self, _mode: &crate::Mode) -> Result<()> {
        assert!(self.source_packet.is_some());

        todo!();
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immutable_source_data_handler_ok() {
        let mut handler = ImmutableSourceDataHandler::default();

        let mut mode = Some(crate::Mode::QUERY);
        while let Some(m) = crate::Mode::next(mode.unwrap()) {
            handler.prepare(&m).unwrap();
            handler
                .handle(&m, &gng_build_shared::MessageType::DATA, "foobar 12345")
                .unwrap();
            handler.verify(&m).unwrap();
            mode = Some(m)
        }
    }
    #[test]
    fn test_immutable_source_data_handler_ok_data_same() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::PREPARE).unwrap();
        handler
            .handle(
                &crate::Mode::PREPARE,
                &gng_build_shared::MessageType::DATA,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::PREPARE).unwrap();

        handler.prepare(&crate::Mode::QUERY).unwrap();
        handler
            .handle(
                &crate::Mode::QUERY,
                &gng_build_shared::MessageType::DATA,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::QUERY).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_no_data_message() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::PREPARE).unwrap();
        handler.verify(&crate::Mode::PREPARE).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_double_data() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::PREPARE).unwrap();
        handler
            .handle(
                &crate::Mode::PREPARE,
                &gng_build_shared::MessageType::DATA,
                "foobar 12345",
            )
            .unwrap();
        handler
            .handle(
                &crate::Mode::PREPARE,
                &gng_build_shared::MessageType::DATA,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::PREPARE).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_non_data() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::PREPARE).unwrap();
        handler
            .handle(
                &crate::Mode::PREPARE,
                &gng_build_shared::MessageType::TEST,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::PREPARE).unwrap();
    }

    #[test]
    #[should_panic(expected = "gng-build-agent did not react as expected!")]
    fn test_immutable_source_data_handler_data_changed() {
        let mut handler = ImmutableSourceDataHandler::default();

        handler.prepare(&crate::Mode::PREPARE).unwrap();
        handler
            .handle(
                &crate::Mode::PREPARE,
                &gng_build_shared::MessageType::DATA,
                "foobar 12345",
            )
            .unwrap();
        handler.verify(&crate::Mode::PREPARE).unwrap();

        handler.prepare(&crate::Mode::QUERY).unwrap();
        handler
            .handle(
                &crate::Mode::QUERY,
                &gng_build_shared::MessageType::DATA,
                "foobar 123456",
            )
            .unwrap();
        handler.verify(&crate::Mode::QUERY).unwrap();
    }
}
