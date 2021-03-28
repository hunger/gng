// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use super::deterministic_directory_iterator::DeterministicDirectoryIterator;

// ----------------------------------------------------------------------
// - MimeTypeDirectoryIterator:
// ----------------------------------------------------------------------

pub struct MimeTypeDirectoryIterator {
    mime_db: xdg_mime::SharedMimeInfo,
    iterator: DeterministicDirectoryIterator,
}

impl MimeTypeDirectoryIterator {
    pub fn new(directory: &std::path::Path) -> gng_shared::Result<Self> {
        Ok(Self {
            mime_db: xdg_mime::SharedMimeInfo::new(),
            iterator: DeterministicDirectoryIterator::new(directory)?,
        })
    }

    fn get_mime_type(&self, in_packet: &mut gng_shared::packet::Path) -> String {
        if let Some(contents) = in_packet.file_contents() {
            let mut guesser = self.mime_db.guess_mime_type();

            match contents {
                gng_shared::packet::FileContents::OnDisk(p) => guesser.path(p),
                gng_shared::packet::FileContents::Buffer(b) => guesser.data(b),
            };
            let guess = guesser.guess();
            guess.mime_type().essence_str().to_string()
        } else if in_packet.is_dir() {
            String::from("inode/directory")
        } else if in_packet.is_link() {
            String::from("inode/symlink")
        } else {
            String::from("error/error")
        }
    }
}

impl Iterator for MimeTypeDirectoryIterator {
    type Item = crate::packager::PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            None => None,
            Some(Err(e)) => Some(Err(e)),
            Some(Ok(p)) => {
                let mut in_packet = p.in_packet;
                let mime_type = self.get_mime_type(&mut in_packet);

                Some(Ok(crate::packager::PacketPath {
                    in_packet,
                    mime_type,
                }))
            }
        }
    }
}
