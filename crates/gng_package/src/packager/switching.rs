// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Packager` that switches between different `Packager`s

use crate::packager::{BoxedPackager, Packager};

// ----------------------------------------------------------------------
// - SwitchingPackager:
// ----------------------------------------------------------------------

/// A `Packager` that can select between a set of `children` `Packager`
pub struct SwitchingPackager {
    children: Vec<BoxedPackager>,
}

impl SwitchingPackager {
    /// Constructor
    #[must_use]
    pub fn new(children: Vec<BoxedPackager>) -> Self {
        Self { children }
    }
}

impl Packager for SwitchingPackager {
    fn package(&mut self, path: &crate::path::Path) -> eyre::Result<bool> {
        for c in &mut self.children {
            if c.package(path)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn finish(&mut self) -> eyre::Result<Vec<std::path::PathBuf>> {
        self.children.iter_mut().fold(Ok(Vec::new()), |acc, p| {
            if let Ok(mut v) = acc {
                v.append(&mut p.finish()?);
                Ok(v)
            } else {
                acc
            }
        })
    }
}
