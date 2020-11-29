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

    /// Verify output of `gng-build-agent` after that has quit successfully:
    fn verify(&mut self, mode: &crate::Mode) -> eyre::Result<()>;
}

// ----------------------------------------------------------------------
// - ImmutableSourceDataHandler:
// ----------------------------------------------------------------------

/// Break out of the
pub struct ImmutableSourceDataHandler {
    source_data: Option<String>, // FIXME: Store a hash to save space!
    was_set_in_current_mode: bool,
}

impl Default for ImmutableSourceDataHandler {
    fn default() -> Self {
        ImmutableSourceDataHandler {
            source_data: None,
            was_set_in_current_mode: false,
        }
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

        if self.was_set_in_current_mode {
            return Err(eyre::eyre!("DATA message received twice in one mode!"));
        }
        self.was_set_in_current_mode = true;

        if self.source_data.is_none() {
            self.source_data = Some(String::from(message));
            return Ok(false);
        }

        if self.source_data.as_ref().expect("was some before!") == message {
            Ok(false)
        } else {
            return Err(eyre::eyre!("Source data changed, aborting!"));
        }
    }

    fn verify(&mut self, _mode: &crate::Mode) -> eyre::Result<()> {
        if self.source_data.is_none() {
            return Err(eyre::eyre!(
                "No source data received during QUERY mode call."
            ));
        }

        self.was_set_in_current_mode = false;

        Ok(())
    }
}
