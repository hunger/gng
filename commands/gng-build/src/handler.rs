// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle events from `gng-build-agent` received via the `CaseOfficer`

mod install_handler;
mod packaging_handler;
mod query_handler;
mod sources_handler;
mod verify_source_packet_handler;

use eyre::Result;

use install_handler::InstallHandler;
use packaging_handler::PackagingHandler;
use query_handler::QueryHandler;
use sources_handler::SourcesHandler;
use verify_source_packet_handler::VerifySourcePacketHandler;

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

type HandlerList = std::rc::Rc<std::cell::RefCell<Vec<Box<dyn Handler>>>>;

fn prepare(handlers: &HandlerList, mode: &crate::Mode) -> eyre::Result<()> {
    let mut handlers = handlers.borrow_mut();
    for h in &mut *handlers {
        h.prepare(mode)?;
    }
    Ok(())
}

fn handle(
    handlers: &HandlerList,
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

fn clean_up(handlers: &HandlerList, mode: &crate::Mode) -> eyre::Result<()> {
    let mut handlers = handlers.borrow_mut();
    for h in &mut *handlers {
        h.clean_up(mode)?;
    }
    Ok(())
}

// ----------------------------------------------------------------------
// - HandlerManager:
// ----------------------------------------------------------------------

/// Constructor
#[must_use]
fn create_handlers(case_officer: &crate::case_officer::CaseOfficer) -> HandlerList {
    let query_handler = Box::<QueryHandler>::default();
    let verify_source_packet_handler = Box::new(VerifySourcePacketHandler::new(
        query_handler.source_packet(),
    ));
    let install_handler = Box::new(InstallHandler::new(
        query_handler.source_packet(),
        &case_officer.root_directory(),
    ));
    let sources_handler = Box::new(SourcesHandler::new(
        query_handler.source_packet(),
        &case_officer.work_directory(),
    ));
    let packaging_handler = Box::new(PackagingHandler::new(
        query_handler.source_packet(),
        &case_officer.install_directory(),
    ));

    let handlers: Vec<Box<dyn Handler>> = vec![
        query_handler,
        verify_source_packet_handler,
        install_handler,
        sources_handler,
        packaging_handler,
    ];

    std::rc::Rc::new(std::cell::RefCell::new(handlers))
}

/// Run `Handler`s using a `CaseOfficer`
///
/// # Errors
/// Return some Error if something  goes wrong.
pub fn run(case_officer: &mut crate::case_officer::CaseOfficer) -> eyre::Result<()> {
    let prepare_handlers = create_handlers(case_officer);
    let handle_handlers = prepare_handlers.clone();
    let clean_up_handlers = prepare_handlers.clone();

    case_officer.process(
        &|m| prepare(&prepare_handlers, m),
        &|m, t, c| handle(&handle_handlers, m, t, c),
        &|m| clean_up(&clean_up_handlers, m),
    )
}
