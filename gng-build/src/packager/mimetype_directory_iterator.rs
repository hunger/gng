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

    fn get_mime_type(
        &self,
        on_disk: &std::path::Path,
        in_packet: &gng_shared::packet::Path,
    ) -> String {
        if in_packet.is_file() {
            let mut guesser = self.mime_db.guess_mime_type();
            let guess = guesser.path(&on_disk).guess();
            guess.mime_type().essence_str().to_string()
        } else {
            String::new()
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
                let on_disk = p.on_disk;
                let in_packet = p.in_packet;
                let mime_type = self.get_mime_type(&on_disk, &in_packet);

                Some(Ok(crate::packager::PacketPath {
                    on_disk,
                    in_packet,
                    mime_type,
                }))
            }
        }
    }
}
