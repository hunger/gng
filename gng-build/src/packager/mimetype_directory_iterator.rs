// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use super::deterministic_directory_iterator::DeterministicDirectoryIterator;

// - Helper:
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// - MimeTypeDirectoryIterator:
// ----------------------------------------------------------------------

pub struct MimeTypeDirectoryIterator {
    iterator: DeterministicDirectoryIterator,
}

impl MimeTypeDirectoryIterator {
    pub fn new(directory: &std::path::Path) -> gng_shared::Result<Self> {
        Ok(Self {
            iterator: DeterministicDirectoryIterator::new(directory)?,
        })
    }
}

impl Iterator for MimeTypeDirectoryIterator {
    type Item = crate::packager::PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}
