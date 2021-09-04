// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle events from `gng-build-agent` received via the `CaseOfficer`

use eyre::Result;

// ----------------------------------------------------------------------
// - Handler:
// ----------------------------------------------------------------------

/// An object used to handle events from the `gng-build-agent`
pub trait Handler {
    /// Verify state before `gng-build-agent` is started
    ///
    /// # Errors
    /// Generic Error
    fn prepare(&mut self, _mode: &crate::Mode) -> Result<()> {
        Ok(())
    }

    /// Handle one message from `gng-build-agent`
    ///
    /// Return `Ok(true)` if this handler handled the message and it does
    /// not need to get passed on to other handlers.
    ///
    /// # Errors
    /// Generic Error
    fn handle(
        &mut self,
        _mode: &crate::Mode,
        _message_type: &gng_build_shared::MessageType,
        _message: &str,
    ) -> Result<bool> {
        Ok(false)
    }

    /// Clean up after `gng-build-agent` has quit successfully
    ///
    /// # Errors
    /// Generic Error
    fn clean_up(&mut self, _mode: &crate::Mode) -> Result<()> {
        Ok(())
    }
}

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

fn prepare(
    handlers: &std::rc::Rc<std::cell::RefCell<Vec<Box<dyn Handler>>>>,
    mode: &crate::Mode,
) -> eyre::Result<()> {
    let mut handlers = handlers.borrow_mut();
    for h in &mut *handlers {
        h.prepare(mode)?;
    }
    Ok(())
}

fn handle(
    handlers: &std::rc::Rc<std::cell::RefCell<Vec<Box<dyn Handler>>>>,
    mode: &crate::Mode,
    message_type: &gng_build_shared::MessageType,
    contents: &str,
) -> eyre::Result<()> {
    tracing::debug!("Handling \"{:?}\": \"{}\".", message_type, contents);

    let mut handlers = handlers.borrow_mut();
    for h in &mut *handlers {
        if h.handle(mode, message_type, contents)? {
            break;
        }
    }
    Ok(())
}

fn clean_up(
    handlers: &std::rc::Rc<std::cell::RefCell<Vec<Box<dyn Handler>>>>,
    mode: &crate::Mode,
) -> eyre::Result<()> {
    let mut handlers = handlers.borrow_mut();
    for h in &mut *handlers {
        h.clean_up(mode)?;
    }
    Ok(())
}

// ----------------------------------------------------------------------
// - HandlerManager:
// ----------------------------------------------------------------------

/// A manager for `Handler`s
pub struct HandlerManager {
    handlers: std::rc::Rc<std::cell::RefCell<Vec<Box<dyn Handler>>>>,
}

impl Default for HandlerManager {
    fn default() -> Self {
        Self {
            handlers: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        }
    }
}

impl HandlerManager {
    /// Run `Handler`s using a `CaseOfficer`
    ///
    /// # Errors
    /// Return some Error if something  goes wrong.
    pub fn run(&mut self, case_officer: &mut crate::case_officer::CaseOfficer) -> eyre::Result<()> {
        let handlers1 = self.handlers.clone();
        let handlers2 = self.handlers.clone();
        let handlers3 = self.handlers.clone();

        case_officer.process(
            &|m| prepare(&handlers1, m),
            &|m, t, c| handle(&handlers2, m, t, c),
            &|m| clean_up(&handlers3, m),
        )
    }
}
