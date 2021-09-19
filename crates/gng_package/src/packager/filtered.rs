// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Packager` that only accepts filtered `Path`

use crate::filter::Filter;
use crate::packager::{BoxedPackager, Packager};

use std::rc::Rc;

// ----------------------------------------------------------------------
// - FilteredPackager:
// ----------------------------------------------------------------------

/// A `Packager` that will only handle certain `Path`
pub struct FilteredPackager {
    /// Debug message for `FilteredPackager`
    pub debug: String,
    filter: Rc<dyn Filter>,
    packager: BoxedPackager,
}

impl FilteredPackager {
    /// Constructor
    pub fn new(debug: String, filter: Rc<dyn Filter>, packager: BoxedPackager) -> Self {
        Self {
            debug,
            filter,
            packager,
        }
    }
}

impl Packager for FilteredPackager {
    #[tracing::instrument(level = "trace", skip(self))]
    fn package(&mut self, path: &crate::path::Path) -> eyre::Result<bool> {
        tracing::trace!("Packaging in {}", self.debug_name());
        if self.filter.matches(path) {
            self.packager.package(path)
        } else {
            Ok(false)
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn finish(&mut self) -> eyre::Result<Vec<std::path::PathBuf>> {
        tracing::trace!("Finishing in {}", self.debug_name());
        self.packager.finish()
    }

    fn debug_name(&self) -> String {
        format!("[ Filtering {} ]", self.debug)
    }
}
