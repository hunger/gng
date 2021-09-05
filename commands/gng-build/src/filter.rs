// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use std::path::Path;

use eyre::Result;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

pub fn strings_to_globs(globs: &[&str]) -> Result<Vec<glob::Pattern>> {
    globs
        .iter()
        .map(|s| glob::Pattern::new(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| eyre::eyre!("Failed to  create GLOB pattern: {}", e))
}

// ----------------------------------------------------------------------
// - Filter:
// ----------------------------------------------------------------------

pub trait Filter {
    fn filter(&self, path: &Path) -> bool;
}

// ----------------------------------------------------------------------
// - GlobFilter:
// ----------------------------------------------------------------------

struct GlobFilter {
    globs: Vec<glob::Pattern>,
}

impl GlobFilter {
    pub fn new(globs: &[&str]) -> Result<Self> {
        Ok(Self {
            globs: strings_to_globs(globs)?,
        })
    }
}

impl Filter for GlobFilter {
    fn filter(&self, path: &Path) -> bool {
        let path = path.to_string_lossy();
        self.globs.iter().any(|p| p.matches(&path))
    }
}

// ----------------------------------------------------------------------
// - AndFilter:
// ----------------------------------------------------------------------

struct AndFilter<L: Filter, R: Filter> {
    left: L,
    right: R,
}

impl<L: Filter, R: Filter> AndFilter<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: Filter, R: Filter> Filter for AndFilter<L, R> {
    fn filter(&self, path: &Path) -> bool {
        self.left.filter(path) && self.right.filter(path)
    }
}

// ----------------------------------------------------------------------
// - OrFilter:
// ----------------------------------------------------------------------

struct OrFilter<L: Filter, R: Filter> {
    left: L,
    right: R,
}

impl<L: Filter, R: Filter> OrFilter<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: Filter, R: Filter> Filter for OrFilter<L, R> {
    fn filter(&self, path: &Path) -> bool {
        self.left.filter(path) || self.right.filter(path)
    }
}

// ----------------------------------------------------------------------
// - AlwaysTrue:
// ----------------------------------------------------------------------

struct AlwaysTrue {}

impl Default for AlwaysTrue {
    fn default() -> Self {
        Self {}
    }
}

impl Filter for AlwaysTrue {
    fn filter(&self, _path: &Path) -> bool {
        true
    }
}

// ----------------------------------------------------------------------
// - AlwaysFalse:
// ----------------------------------------------------------------------

struct AlwaysFalse {}

impl Default for AlwaysFalse {
    fn default() -> Self {
        Self {}
    }
}

impl Filter for AlwaysFalse {
    fn filter(&self, _path: &Path) -> bool {
        false
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{AlwaysFalse, AlwaysTrue, AndFilter, Filter, GlobFilter, OrFilter};

    use std::path::Path;

    // ----------------------------------------------------------------------
    // - Tests:
    // ----------------------------------------------------------------------

    // Name:
    #[test]
    fn and_filter() {
        assert!(
            !AndFilter::new(AlwaysFalse::default(), AlwaysFalse::default())
                .filter(&Path::new("/usr/foo"))
        );
        assert!(
            !AndFilter::new(AlwaysFalse::default(), AlwaysTrue::default())
                .filter(&Path::new("/usr/foo"))
        );
        assert!(
            !AndFilter::new(AlwaysTrue::default(), AlwaysFalse::default())
                .filter(&Path::new("/usr/foo"))
        );
        assert!(AndFilter::new(AlwaysTrue::default(), AlwaysTrue::default())
            .filter(&Path::new("/usr/foo")));
    }

    // Name:
    #[test]
    fn or_filter() {
        assert!(
            !OrFilter::new(AlwaysFalse::default(), AlwaysFalse::default())
                .filter(&Path::new("/usr/foo"))
        );
        assert!(OrFilter::new(AlwaysFalse::default(), AlwaysTrue::default())
            .filter(&Path::new("/usr/foo")));
        assert!(OrFilter::new(AlwaysTrue::default(), AlwaysFalse::default())
            .filter(&Path::new("/usr/foo")));
        assert!(OrFilter::new(AlwaysTrue::default(), AlwaysTrue::default())
            .filter(&Path::new("/usr/foo")));
    }
}
