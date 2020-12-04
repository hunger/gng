// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle messages from `gng-build-agent`

use sha3::{Digest, Sha3_256};

// - Helper:
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// - Message Handler:
// ----------------------------------------------------------------------

/// An object used to handle messages from the `gng-build-agent`
pub trait MessageHandler {
    /// Verify output of `gng-build-agent` after that has quit successfully:
    fn prepare(&mut self, mode: &crate::Mode) -> eyre::Result<()>;

    /// Handle one message
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool>;

    /// Verify output of `gng-build-agent` after that has quit successfully:
    fn verify(&mut self, mode: &crate::Mode) -> eyre::Result<()>;
}

// ----------------------------------------------------------------------
// - ImmutableSourceDataHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct ImmutableSourceDataHandler {
    hash: Option<Vec<u8>>,
    first_message: bool,
}

impl ImmutableSourceDataHandler {
    fn hash_message(&mut self, message: &str) -> Vec<u8> {
        let mut hasher = Sha3_256::new();
        hasher.update(message.as_bytes());
        let mut v = Vec::with_capacity(Sha3_256::output_size());
        v.extend_from_slice(&hasher.finalize());

        v
    }
}

impl Default for ImmutableSourceDataHandler {
    fn default() -> Self {
        ImmutableSourceDataHandler {
            hash: None,
            first_message: true,
        }
    }
}

impl MessageHandler for ImmutableSourceDataHandler {
    fn prepare(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        self.first_message = true;
        Ok(())
    }

    fn handle(
        &mut self,
        _mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool> {
        if message_type != &gng_build_shared::MessageType::DATA {
            self.first_message = false;
            return Ok(false);
        }

        if !self.first_message {
            return Err(eyre::eyre!(
                "The build agent did not send a DATA message first!"
            ));
        }

        self.first_message = false;

        let v = self.hash_message(&message);

        match self.hash.as_ref() {
            None => {
                self.hash = Some(v);
                return Ok(false);
            }
            Some(vg) if *vg == v => {
                return Ok(false);
            }
            Some(_) => {
                return Err(eyre::eyre!("Source data changed, aborting!"));
            }
        }
    }

    fn verify(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        if self.first_message {
            return Err(eyre::eyre!("The build agent did not send any message!"));
        }

        if self.hash.is_none() {
            return Err(eyre::eyre!(
                "No source data received during QUERY mode call."
            ));
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - ValidateInputHandler:
// ----------------------------------------------------------------------

/// Make sure the source as seen by the `gng-build-agent` stays constant
pub struct ValidateInputHandler {}

impl Default for ValidateInputHandler {
    fn default() -> Self {
        ValidateInputHandler {}
    }
}

impl MessageHandler for ValidateInputHandler {
    fn prepare(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        Ok(())
    }

    fn handle(
        &mut self,
        _mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool> {
        if message_type != &gng_build_shared::MessageType::DATA {
            return Ok(false);
        }

        Ok(false)
    }

    fn verify(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        Ok(())
    }
}
