// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! `Filter` objects used to find the right package for a file to be stored in.

use crate::path::Path;

// ----------------------------------------------------------------------
// - Filter:
// ----------------------------------------------------------------------

/// The `Filter` trait
pub trait Filter {
    /// Check whether a `path` matches and should thus go into the `Packet`
    /// or not.
    fn matches(&self, path: &Path) -> bool;
}

// ----------------------------------------------------------------------
// - GlobFilter:
// ----------------------------------------------------------------------

/// A Filter that applies a set of `glob::Pattern`
pub struct GlobFilter {
    globs: Vec<glob::Pattern>,
}

impl GlobFilter {
    /// Constructor
    #[must_use]
    pub const fn new(globs: Vec<glob::Pattern>) -> Self {
        Self { globs }
    }
}

impl Filter for GlobFilter {
    fn matches(&self, path: &Path) -> bool {
        path.as_path()
            .to_str()
            .map_or(false, |path| self.globs.iter().any(|p| p.matches(path)))
    }
}

// ----------------------------------------------------------------------
// - AndFilter:
// ----------------------------------------------------------------------

/// Requires both `left` and `right` to match
pub struct AndFilter<L: Filter, R: Filter> {
    left: L,
    right: R,
}

impl<L: Filter, R: Filter> AndFilter<L, R> {
    /// Constructor
    #[must_use]
    pub const fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: Filter, R: Filter> Filter for AndFilter<L, R> {
    fn matches(&self, path: &Path) -> bool {
        self.left.matches(path) && self.right.matches(path)
    }
}

// ----------------------------------------------------------------------
// - OrFilter:
// ----------------------------------------------------------------------

/// A `Filter` that matches when `left` or `right` matches.
pub struct OrFilter<L: Filter, R: Filter> {
    left: L,
    right: R,
}

impl<L: Filter, R: Filter> OrFilter<L, R> {
    /// Constructor
    #[must_use]
    pub const fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: Filter, R: Filter> Filter for OrFilter<L, R> {
    fn matches(&self, path: &Path) -> bool {
        self.left.matches(path) || self.right.matches(path)
    }
}

// ----------------------------------------------------------------------
// - AlwaysTrue:
// ----------------------------------------------------------------------

/// A `Filter` that always matches.
#[derive(Default)]
pub struct AlwaysTrue {}

impl Filter for AlwaysTrue {
    fn matches(&self, _path: &Path) -> bool {
        true
    }
}

// ----------------------------------------------------------------------
// - AlwaysFalse:
// ----------------------------------------------------------------------

/// A `Filter` that never matches
#[derive(Default)]
pub struct AlwaysFalse {}

impl Filter for AlwaysFalse {
    fn matches(&self, _path: &Path) -> bool {
        false
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{AlwaysFalse, AlwaysTrue, AndFilter, Filter, OrFilter};

    use crate::path::Path;

    // ----------------------------------------------------------------------
    // - Tests:
    // ----------------------------------------------------------------------

    fn path(input: &str) -> Path {
        Path::new_file_from_disk(
            std::path::Path::new("/tmp/foo"),
            std::path::Path::new(input),
            0o755,
            0,
            0,
            42,
        )
    }

    // Name:
    #[test]
    fn and_filter() {
        assert!(
            !AndFilter::new(AlwaysFalse::default(), AlwaysFalse::default())
                .matches(&path("/usr/foo"))
        );
        assert!(
            !AndFilter::new(AlwaysFalse::default(), AlwaysTrue::default())
                .matches(&path("/usr/foo"))
        );
        assert!(
            !AndFilter::new(AlwaysTrue::default(), AlwaysFalse::default())
                .matches(&path("/usr/foo"))
        );
        assert!(
            AndFilter::new(AlwaysTrue::default(), AlwaysTrue::default()).matches(&path("/usr/foo"))
        );
    }

    // Name:
    #[test]
    fn or_filter() {
        assert!(
            !OrFilter::new(AlwaysFalse::default(), AlwaysFalse::default())
                .matches(&path("/usr/foo"))
        );
        assert!(
            OrFilter::new(AlwaysFalse::default(), AlwaysTrue::default()).matches(&path("/usr/foo"))
        );
        assert!(
            OrFilter::new(AlwaysTrue::default(), AlwaysFalse::default()).matches(&path("/usr/foo"))
        );
        assert!(
            OrFilter::new(AlwaysTrue::default(), AlwaysTrue::default()).matches(&path("/usr/foo"))
        );
    }
}
