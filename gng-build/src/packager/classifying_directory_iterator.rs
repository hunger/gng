// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use super::deterministic_directory_iterator::DeterministicDirectoryIterator;

// ----------------------------------------------------------------------
// - ClassifyingDirectoryIterator:
// ----------------------------------------------------------------------

pub struct ClassifyingDirectoryIterator {
    cookie: filemagic::Magic,
    iterator: DeterministicDirectoryIterator,
}

impl ClassifyingDirectoryIterator {
    pub fn new(directory: &std::path::Path) -> eyre::Result<Self> {
        let cookie = filemagic::Magic::open(filemagic::flags::Flags::default()).map_err(|e| {
            gng_shared::Error::Runtime {
                message: format!("File type detection setup failed: {}", e),
            }
        })?;
        cookie
            .load::<String>(&[])
            .map_err(|e| gng_shared::Error::Runtime {
                message: format!("File type detection database failed to load: {}", e),
            })?;
        Ok(Self {
            cookie,
            iterator: DeterministicDirectoryIterator::new(directory)?,
        })
    }

    fn get_mime_type(&self, in_packet: &mut gng_shared::packet::Path) -> String {
        if let Some(contents) = in_packet.file_contents() {
            match contents {
                gng_shared::packet::FileContents::OnDisk(p) => {
                    self.cookie.file(p).unwrap_or_default()
                }
                gng_shared::packet::FileContents::Buffer(b) => {
                    self.cookie.buffer(b).unwrap_or_default()
                }
            }
        } else if in_packet.is_dir() {
            String::from("directory")
        } else if in_packet.is_link() {
            String::from("symlink")
        } else {
            String::from("<UNKNOWN>")
        }
    }
}

impl Iterator for ClassifyingDirectoryIterator {
    type Item = crate::packager::PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            None => None,
            Some(Err(e)) => Some(Err(e)),
            Some(Ok(p)) => {
                println!("MIME_iterator: {:?}.", p);
                let mut in_packet = p.in_packet;
                let mime_type = self.get_mime_type(&mut in_packet);

                Some(Ok(crate::packager::PacketPath {
                    in_packet,
                    classification: mime_type,
                }))
            }
        }
    }
}
