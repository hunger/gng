// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle messages from `gng-build-agent`

// - Helper:
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// - Message Handler:
// ----------------------------------------------------------------------

/// An object used to handle messages from the `gng-build-agent`
pub trait MessageHandler {
    /// Handle one message
    fn handle(
        &mut self,
        mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool>;
}

// ----------------------------------------------------------------------
// - ImmutableSourceDataHandler:
// ----------------------------------------------------------------------

/// Break out of the
pub struct ImmutableSourceDataHandler {
    source_data: Option<String>,
}

impl Default for ImmutableSourceDataHandler {
    fn default() -> Self {
        ImmutableSourceDataHandler { source_data: None }
    }
}

impl MessageHandler for ImmutableSourceDataHandler {
    fn handle(
        &mut self,
        _mode: &crate::Mode,
        message_type: &gng_build_shared::MessageType,
        message: &str,
    ) -> eyre::Result<bool> {
        if message_type != &gng_build_shared::MessageType::DATA {
            return Ok(false);
        }

        if self.source_data.is_none() {
            self.source_data = Some(String::from(message));
            return Ok(false);
        }

        if self.source_data.as_ref().expect("was some before!") == message {
            Ok(false)
        } else {
            return Err(eyre::eyre!("Source package data changed, aborting!"));
        }
    }
}
